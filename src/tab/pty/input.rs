use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};

const INPUT_QUEUE_MESSAGES: usize = 256;

#[derive(Debug)]
pub(in crate::tab) enum PtyInput {
    User(Vec<u8>),
    Query(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tab) enum InputQueueResult {
    Queued,
    Full,
    Disconnected,
}

pub(super) struct PtyInputChannel {
    tx: SyncSender<PtyInput>,
    query_in_flight: Arc<AtomicBool>,
    _writer: JoinHandle<()>,
}

impl PtyInputChannel {
    pub fn spawn(writer: Box<dyn Write + Send>, id: u64) -> Result<Self> {
        let query_in_flight = Arc::new(AtomicBool::new(false));
        let writer_flag = Arc::clone(&query_in_flight);
        let (tx, rx) = mpsc::sync_channel(INPUT_QUEUE_MESSAGES);
        let handle = thread::Builder::new()
            .name(format!("mux-pty-write-{id}"))
            .spawn(move || writer_loop(writer, rx, writer_flag))
            .context("spawn pty writer thread")?;
        Ok(Self {
            tx,
            query_in_flight,
            _writer: handle,
        })
    }

    pub fn queue_user(&self, bytes: Vec<u8>) -> InputQueueResult {
        queue_result(self.tx.try_send(PtyInput::User(bytes)))
    }

    pub fn queue_query(&self, bytes: Vec<u8>) -> InputQueueResult {
        if self
            .query_in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return InputQueueResult::Full;
        }
        let result = queue_result(self.tx.try_send(PtyInput::Query(bytes)));
        if result != InputQueueResult::Queued {
            self.query_in_flight.store(false, Ordering::Release);
        }
        result
    }
}

fn queue_result(result: Result<(), TrySendError<PtyInput>>) -> InputQueueResult {
    match result {
        Ok(()) => InputQueueResult::Queued,
        Err(TrySendError::Full(_)) => InputQueueResult::Full,
        Err(TrySendError::Disconnected(_)) => InputQueueResult::Disconnected,
    }
}

pub(in crate::tab) fn writer_loop(
    mut writer: Box<dyn Write + Send>,
    rx: Receiver<PtyInput>,
    query_in_flight: Arc<AtomicBool>,
) {
    while let Ok(input) = rx.recv() {
        let (bytes, is_query) = match input {
            PtyInput::User(bytes) => (bytes, false),
            PtyInput::Query(bytes) => (bytes, true),
        };
        let result = writer.write_all(&bytes).and_then(|_| writer.flush());
        if is_query {
            query_in_flight.store(false, Ordering::Release);
        }
        if result.is_err() {
            break;
        }
    }
    query_in_flight.store(false, Ordering::Release);
}

impl super::PtyTransport {
    pub fn queue_user(&self, bytes: Vec<u8>) -> InputQueueResult {
        self.input.queue_user(bytes)
    }

    pub fn queue_query(&self, bytes: Vec<u8>) -> InputQueueResult {
        self.input.queue_query(bytes)
    }
}
