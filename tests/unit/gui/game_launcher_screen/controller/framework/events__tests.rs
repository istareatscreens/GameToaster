use std::sync::mpsc;

use super::super::buttons::Button;
use super::*;

#[test]
fn coalesces_unchanged_snapshots_and_forwards_distinct_stream() {
    let (tx, rx) = mpsc::sync_channel(64);
    let sink = EventSink::new(tx, None);
    let a = ButtonWord::from_bits(Button::Ba4.mask());
    let b = ButtonWord::EMPTY;

    sink.try_send(SessionEvent::Connected).unwrap();
    sink.try_send(SessionEvent::Snapshot(a)).unwrap();
    sink.try_send(SessionEvent::Snapshot(a)).unwrap();
    sink.try_send(SessionEvent::Snapshot(a)).unwrap();
    sink.try_send(SessionEvent::Snapshot(b)).unwrap();
    sink.try_send(SessionEvent::Snapshot(b)).unwrap();
    sink.try_send(SessionEvent::Snapshot(a)).unwrap();
    sink.try_send(SessionEvent::Disconnected).unwrap();

    let forwarded: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(forwarded.len(), 5, "duplicates must be coalesced away");
    assert!(matches!(forwarded[0], SessionEvent::Connected));
    assert!(matches!(forwarded[1], SessionEvent::Snapshot(w) if w == a));
    assert!(matches!(forwarded[2], SessionEvent::Snapshot(w) if w == b));
    assert!(matches!(forwarded[3], SessionEvent::Snapshot(w) if w == a));
    assert!(matches!(forwarded[4], SessionEvent::Disconnected));
}

#[test]
fn steady_no_change_stream_does_not_saturate_the_bounded_channel() {
    let (tx, rx) = mpsc::sync_channel(4);
    let sink = EventSink::new(tx, None);
    let held = ButtonWord::from_bits(Button::Ba1.mask());
    for _ in 0..1000 {
        sink.try_send(SessionEvent::Snapshot(held)).unwrap();
    }
    let forwarded: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(forwarded.len(), 1);
}
