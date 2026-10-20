use super::*;

#[test]
fn cancelled_turns_true_when_the_transport_is_cancelled() {
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
    let transport = MxjvsTransport {
        device: event(),
        cancellation: event(),
        io_event: event(),
    };
    assert!(!transport.cancelled());
    transport.cancel();
    assert!(transport.cancelled());
    assert!(transport.cancelled(), "the event stays set");
}
