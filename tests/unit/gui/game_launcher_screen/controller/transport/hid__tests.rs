use super::*;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

fn candidate(vid: u16, pid: u16, len: usize) -> HidCandidate {
    HidCandidate {
        path: vec![0],
        vid,
        pid,
        input_report_len: len,
    }
}

#[test]
fn selection_is_exact_validates_length_and_rejects_ambiguity() {
    let wanted = HidIdentity {
        vid: 0x0CA3,
        pid: 0x0021,
        minimum_input_report_len: 31,
    };
    assert_eq!(
        select_matching(vec![candidate(0x0CA3, 0x0021, 64)], &wanted)
            .unwrap()
            .input_report_len,
        64
    );
    assert!(select_matching(vec![candidate(0x0CA3, 0x0021, 30)], &wanted).is_err());
    assert!(select_matching(
        vec![candidate(0x0CA3, 0x0021, 64), candidate(0x0CA3, 0x0021, 64)],
        &wanted,
    )
    .unwrap_err()
    .contains("multiple"));
}

#[test]
fn blocked_reader_cancels_joins_and_releases_every_tracked_resource() {
    let audit = ResourceAudit::new();
    let event = || {
        Arc::new(
            TrackedHandle::new(
                unsafe { CreateEventW(None, true, false, None).unwrap() },
                &audit,
            )
            .unwrap(),
        )
    };
    let device = event();
    let cancellation = event();
    let read_event = event();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let blocked_on = cancellation.clone();
    let thread_audit = audit.clone();
    let reader = thread::spawn(move || {
        let _guard = ReaderGuard::new(&thread_audit);
        unsafe {
            let _ = WaitForSingleObject(blocked_on.raw(), INFINITE);
        }
        let _ = done_tx.send(());
    });
    let mut session = HidSession {
        device: Some(device),
        cancellation: Some(cancellation),
        read_event: Some(read_event),
        reader: Some(reader),
        done: done_rx,
    };
    session.disconnect(Duration::from_secs(1)).unwrap();
    assert_eq!(audit.counts(), (0, 0));
}
