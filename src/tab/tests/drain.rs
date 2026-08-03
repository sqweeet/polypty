use std::sync::mpsc::{self, TrySendError};
use std::thread;

use crate::tab::pty::output::{drain_output, DrainOutcome};

#[test]
fn drain_respects_budget_and_preserves_chunk_tail() {
    let (tx, rx) = mpsc::sync_channel(2);
    tx.send((0u8..10).collect()).unwrap();
    drop(tx);
    let mut pending = None;
    let mut output = Vec::new();

    let first = drain_output(&rx, &mut pending, 4, |bytes| {
        output.extend_from_slice(bytes)
    });
    assert_eq!(
        first,
        DrainOutcome {
            bytes: 4,
            disconnected: false,
        }
    );
    assert!(pending.is_some());
    let second = drain_output(&rx, &mut pending, 4, |bytes| {
        output.extend_from_slice(bytes)
    });
    assert_eq!(
        second,
        DrainOutcome {
            bytes: 4,
            disconnected: false,
        }
    );
    let third = drain_output(&rx, &mut pending, 4, |bytes| {
        output.extend_from_slice(bytes)
    });
    assert_eq!(
        third,
        DrainOutcome {
            bytes: 2,
            disconnected: true,
        }
    );
    assert_eq!(output, (0u8..10).collect::<Vec<_>>());
}

#[test]
fn bounded_output_channel_applies_backpressure() {
    let (tx, rx) = mpsc::sync_channel(1);
    tx.try_send(vec![1]).unwrap();
    assert!(matches!(tx.try_send(vec![2]), Err(TrySendError::Full(_))));
    assert_eq!(rx.recv().unwrap(), vec![1]);
    tx.try_send(vec![2]).unwrap();
    assert_eq!(rx.recv().unwrap(), vec![2]);
}

#[test]
fn budgeted_drains_do_not_lose_sustained_output() {
    let expected: Vec<u8> = (0..512 * 1024).map(|i| (i % 251) as u8).collect();
    let producer_bytes = expected.clone();
    let (tx, rx) = mpsc::sync_channel(2);
    let producer = thread::spawn(move || {
        for chunk in producer_bytes.chunks(8 * 1024) {
            tx.send(chunk.to_vec()).unwrap();
        }
    });
    let mut pending = None;
    let mut actual = Vec::with_capacity(expected.len());
    loop {
        let outcome = drain_output(&rx, &mut pending, 997, |bytes| {
            actual.extend_from_slice(bytes)
        });
        assert!(outcome.bytes <= 997);
        if outcome.disconnected {
            break;
        }
        if outcome.bytes == 0 {
            thread::yield_now();
        }
    }
    producer.join().unwrap();
    assert_eq!(actual, expected);
}
