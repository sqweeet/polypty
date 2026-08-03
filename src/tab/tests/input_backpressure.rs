use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::tab::pty::input::{writer_loop, PtyInput};

struct GateWriter {
    entered: SyncSender<()>,
    release: Option<Receiver<()>>,
}

impl Write for GateWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Some(release) = self.release.take() {
            self.entered.send(()).unwrap();
            release.recv().unwrap();
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn blocked_pty_writer_does_not_block_input_producers() {
    let (input_tx, input_rx) = mpsc::sync_channel(2);
    let (entered_tx, entered_rx) = mpsc::sync_channel(0);
    let (release_tx, release_rx) = mpsc::sync_channel(0);
    let query_in_flight = Arc::new(AtomicBool::new(true));
    let writer_flag = Arc::clone(&query_in_flight);
    let writer = thread::spawn(move || {
        writer_loop(
            Box::new(GateWriter {
                entered: entered_tx,
                release: Some(release_rx),
            }),
            input_rx,
            writer_flag,
        )
    });

    input_tx.try_send(PtyInput::Query(vec![1])).unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    input_tx.try_send(PtyInput::User(vec![2])).unwrap();
    input_tx.try_send(PtyInput::User(vec![3])).unwrap();
    assert!(matches!(
        input_tx.try_send(PtyInput::User(vec![4])),
        Err(TrySendError::Full(_))
    ));
    release_tx.send(()).unwrap();
    drop(input_tx);
    writer.join().unwrap();
    assert!(!query_in_flight.load(Ordering::Acquire));
}
