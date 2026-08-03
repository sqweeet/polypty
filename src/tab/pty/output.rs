use std::io::Read;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};

const READ_CHUNK_BYTES: usize = 8 * 1024;
const QUEUE_CHUNKS: usize = 32;
const POLL_BYTE_BUDGET: usize = 32 * 1024;

#[derive(Debug)]
pub(in crate::tab) struct PendingOutput {
    bytes: Vec<u8>,
    offset: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::tab) struct DrainOutcome {
    pub bytes: usize,
    pub disconnected: bool,
}

pub(super) struct PtyOutputChannel {
    rx: Receiver<Vec<u8>>,
    pending: Option<PendingOutput>,
    _reader: JoinHandle<()>,
    disconnected: bool,
    last_poll_bytes: usize,
}

impl PtyOutputChannel {
    pub fn spawn(mut reader: Box<dyn Read + Send>, id: u64) -> Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(QUEUE_CHUNKS);
        let handle = thread::Builder::new()
            .name(format!("polypty-pty-read-{id}"))
            .spawn(move || reader_loop(&mut reader, tx))
            .context("spawn pty reader thread")?;
        Ok(Self {
            rx,
            pending: None,
            _reader: handle,
            disconnected: false,
            last_poll_bytes: 0,
        })
    }

    pub fn drain(&mut self, consume: impl FnMut(&[u8])) -> DrainOutcome {
        let outcome = drain_output(&self.rx, &mut self.pending, POLL_BYTE_BUDGET, consume);
        self.last_poll_bytes = outcome.bytes;
        self.disconnected |= outcome.disconnected;
        outcome
    }

    pub fn is_fully_disconnected(&self) -> bool {
        self.disconnected && self.pending.is_none()
    }
}

fn reader_loop(reader: &mut dyn Read, tx: mpsc::SyncSender<Vec<u8>>) {
    let mut buf = [0u8; READ_CHUNK_BYTES];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) if tx.send(buf[..n].to_vec()).is_err() => break,
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break,
        }
    }
}

pub(in crate::tab) fn drain_output(
    rx: &Receiver<Vec<u8>>,
    pending: &mut Option<PendingOutput>,
    budget: usize,
    mut consume: impl FnMut(&[u8]),
) -> DrainOutcome {
    let mut outcome = DrainOutcome::default();
    while outcome.bytes < budget {
        if pending.is_none() {
            match rx.try_recv() {
                Ok(bytes) if !bytes.is_empty() => {
                    *pending = Some(PendingOutput { bytes, offset: 0 });
                }
                Ok(_) => continue,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    outcome.disconnected = true;
                    break;
                }
            }
        }
        let Some(chunk) = pending.as_mut() else {
            continue;
        };
        let take = (chunk.bytes.len() - chunk.offset).min(budget - outcome.bytes);
        consume(&chunk.bytes[chunk.offset..chunk.offset + take]);
        chunk.offset += take;
        outcome.bytes += take;
        if chunk.offset == chunk.bytes.len() {
            *pending = None;
        }
    }
    outcome
}

impl super::PtyTransport {
    pub fn drain_output(&mut self, consume: impl FnMut(&[u8])) -> DrainOutcome {
        self.output.drain(consume)
    }

    pub fn last_poll_bytes(&self) -> usize {
        self.output.last_poll_bytes
    }
}
