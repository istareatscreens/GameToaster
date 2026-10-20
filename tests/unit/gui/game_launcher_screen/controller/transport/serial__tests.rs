use super::*;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

#[test]
fn a_port_name_becomes_its_device_path_and_nothing_else_does() {
    assert_eq!(com_port_path("COM4").as_deref(), Some(r"\\.\COM4"));
    assert_eq!(com_port_path(" com21 ").as_deref(), Some(r"\\.\COM21"));
    assert_eq!(com_port_path("COM0"), None);
    assert_eq!(com_port_path("COM4junk"), None);
    assert_eq!(com_port_path(r"\\.\mxjvs"), None);
    assert_eq!(com_port_path(""), None);
}

#[test]
fn a_port_another_program_holds_is_in_use_and_anything_else_is_a_failure() {
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_SHARING_VIOLATION,
    };
    let error = |code: windows::Win32::Foundation::WIN32_ERROR| {
        windows::core::Error::from(HRESULT::from_win32(code.0))
    };
    assert_eq!(
        open_error(r"\\.\COM21", error(ERROR_ACCESS_DENIED)),
        SerialOpenError::InUse
    );
    assert_eq!(
        open_error(r"\\.\COM21", error(ERROR_SHARING_VIOLATION)),
        SerialOpenError::InUse
    );
    let missing = open_error(r"\\.\COM99", error(ERROR_FILE_NOT_FOUND));
    assert!(
        matches!(&missing, SerialOpenError::Failed(message) if message.starts_with(r"failed to open serial port \\.\COM99: ")),
        "{missing:?}"
    );
    assert_eq!(
        String::from(SerialOpenError::InUse),
        "the serial port is in use by another program"
    );
}

#[test]
fn a_port_that_does_not_exist_fails_to_open() {
    let audit = ResourceAudit::new();
    let result = SerialTransport::open(r"\\.\COM255", &audit);
    assert!(
        matches!(result, Err(SerialOpenError::Failed(_))),
        "COM255 must not exist on a test machine"
    );
    assert_eq!(audit.counts().0, 0, "no handle is left open");
}

#[test]
fn cancellation_interrupts_a_pending_fake_endpoint_and_releases_handles() {
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
    let transport = SerialTransport {
        device: event(),
        cancellation: event(),
        io_event: event(),
    };
    transport.cancel();
    assert_eq!(
        unsafe { WaitForSingleObject(transport.cancellation.raw(), 0) },
        WAIT_OBJECT_0
    );
    let overlapped = OVERLAPPED {
        hEvent: transport.io_event.raw(),
        ..Default::default()
    };
    assert!(transport
        .complete_overlapped(
            Err(windows::core::Error::from_hresult(HRESULT::from_win32(
                ERROR_IO_PENDING.0,
            ))),
            &overlapped,
            Duration::from_secs(1),
            "read",
        )
        .unwrap_err()
        .contains("cancelled"));
    drop(transport);
    assert_eq!(audit.counts(), (0, 0));
}

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
    let transport = SerialTransport {
        device: event(),
        cancellation: event(),
        io_event: event(),
    };
    assert!(!transport.cancelled());
    transport.cancel();
    assert!(transport.cancelled());
    assert!(transport.cancelled(), "the event stays set");
}
