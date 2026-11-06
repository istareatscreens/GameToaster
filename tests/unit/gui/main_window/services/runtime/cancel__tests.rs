use windows::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::WaitForSingleObject;

use super::*;

fn signalled(cancel: &LaunchCancel) -> bool {
    let event = cancel.event().expect("the event is created");
    unsafe { WaitForSingleObject(event, 0) == WAIT_OBJECT_0 }
}

#[test]
fn a_new_cancel_is_not_cancelled_and_its_event_is_unsignalled() {
    let cancel = LaunchCancel::new();
    assert!(!cancel.is_cancelled());
    let event = cancel.event().expect("the event is created");
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_TIMEOUT);
}

#[test]
fn cancelling_marks_it_signals_the_event_and_is_seen_through_every_clone() {
    let cancel = LaunchCancel::new();
    let worker_side = cancel.clone();
    cancel.cancel();
    assert!(worker_side.is_cancelled());
    assert!(signalled(&worker_side));

    cancel.cancel();
    assert!(worker_side.is_cancelled());
}

#[test]
fn a_registered_operation_is_cancelled_with_the_launch() {
    let cancel = LaunchCancel::new();
    let operation = OperationCancel::standalone();
    cancel.set_operation(Some(operation.clone()));
    assert!(!operation.was_cancelled());
    cancel.cancel();
    assert!(operation.was_cancelled());
}

#[test]
fn an_operation_registered_after_the_cancel_is_cancelled_at_once() {
    let cancel = LaunchCancel::new();
    cancel.cancel();
    let operation = OperationCancel::standalone();
    cancel.set_operation(Some(operation.clone()));
    assert!(operation.was_cancelled());
}

#[test]
fn an_operation_that_ended_is_not_cancelled_later() {
    let cancel = LaunchCancel::new();
    let operation = OperationCancel::standalone();
    cancel.set_operation(Some(operation.clone()));
    cancel.set_operation(None);
    cancel.cancel();
    assert!(!operation.was_cancelled());
}

#[test]
fn the_debug_form_says_whether_it_is_cancelled() {
    let cancel = LaunchCancel::new();
    assert_eq!(format!("{cancel:?}"), "LaunchCancel { cancelled: false }");
    cancel.cancel();
    assert_eq!(format!("{cancel:?}"), "LaunchCancel { cancelled: true }");
}
