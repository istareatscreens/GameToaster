use super::*;
use windows::Win32::System::Threading::CreateEventW;

#[test]
fn tracked_resources_count_against_their_audit_and_return_on_drop() {
    let audit = ResourceAudit::new();
    {
        let event = unsafe { CreateEventW(None, true, false, None) }.unwrap();
        let _handle = TrackedHandle::new(event, &audit).unwrap();
        let _reader = ReaderGuard::new(&audit);
        assert_eq!(audit.counts(), (1, 1));
    }
    assert_eq!(audit.counts(), (0, 0));
}

#[test]
fn invalid_handles_never_change_the_audited_count() {
    let audit = ResourceAudit::new();
    assert!(TrackedHandle::new(HANDLE::default(), &audit).is_err());
    assert!(TrackedHandle::new(INVALID_HANDLE_VALUE, &audit).is_err());
    assert_eq!(audit.counts(), (0, 0));
}

#[test]
fn a_resource_counts_only_against_the_audit_it_was_opened_under() {
    let first = ResourceAudit::new();
    let second = ResourceAudit::new();
    let event = unsafe { CreateEventW(None, true, false, None) }.unwrap();
    let handle = TrackedHandle::new(event, &first).unwrap();
    let reader = ReaderGuard::new(&second);
    assert_eq!(first.counts(), (1, 0));
    assert_eq!(second.counts(), (0, 1));
    drop(handle);
    drop(reader);
    assert_eq!((first.counts(), second.counts()), ((0, 0), (0, 0)));
}
