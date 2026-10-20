use super::*;

#[test]
fn standard_handles_are_restored_verbatim() {
    unsafe {
        let before = StandardHandles::capture();
        let bogus = HANDLE(0x7A5E_usize as *mut core::ffi::c_void);
        let _ = SetStdHandle(STD_INPUT_HANDLE, bogus);
        assert_eq!(standard_handle(STD_INPUT_HANDLE), bogus);
        before.restore();
        assert_eq!(standard_handle(STD_INPUT_HANDLE), before.input);
        assert_eq!(standard_handle(STD_OUTPUT_HANDLE), before.output);
        assert_eq!(standard_handle(STD_ERROR_HANDLE), before.error);
    }
}

#[test]
fn a_target_that_cannot_be_attached_is_not_handled_and_changes_nothing() {
    unsafe {
        let before = StandardHandles::capture();
        assert!(!hide_console_for_pid(std::process::id()));
        assert!(!hide_console_for_pid(0));
        assert_eq!(standard_handle(STD_INPUT_HANDLE), before.input);
        assert_eq!(standard_handle(STD_OUTPUT_HANDLE), before.output);
        assert_eq!(standard_handle(STD_ERROR_HANDLE), before.error);
    }
}
