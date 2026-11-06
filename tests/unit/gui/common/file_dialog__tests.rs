use super::*;

#[test]
fn a_window_of_this_process_owns_the_picker() {
    let owner = owner_for(0x0001_2345, 77, 77).expect("our own window owns it");
    let handle = owner
        .window_handle()
        .expect("the owner has a window handle");
    match handle.as_raw() {
        RawWindowHandle::Win32(win32) => assert_eq!(win32.hwnd.get(), 0x0001_2345),
        other => panic!("a Win32 handle, not {other:?}"),
    }
    assert!(owner.display_handle().is_ok());
}

#[test]
fn another_programs_window_owns_nothing() {
    assert_eq!(owner_for(0x0001_2345, 12, 77), None);
}

#[test]
fn no_foreground_window_owns_nothing() {
    assert_eq!(owner_for(0, 77, 77), None);
}

#[test]
fn a_picker_opens_whatever_is_in_front() {
    let _ = file_dialog();
    assert_eq!(
        owner_for(1, std::process::id() + 1, std::process::id()),
        None
    );
}

#[test]
fn a_picker_is_centred_over_its_owner() {
    let (x, y) = centred_in((0, 0, 600, 400), (100, 100, 1100, 900), (0, 0, 1920, 1040));
    assert_eq!((x + 300, y + 200), (600, 500));
}

#[test]
fn a_picker_that_would_leave_the_work_area_moves_only_as_far_as_needed() {
    let work = (0, 0, 1920, 1040);
    let (x, y) = centred_in((0, 0, 600, 400), (1500, 800, 2100, 1300), work);
    assert_eq!((x, y), (1920 - 600, 1040 - 400));

    let work = (1920, 0, 3000, 1880);
    let (x, y) = centred_in((0, 0, 600, 400), (1800, -100, 2000, 100), work);
    assert_eq!((x, y), (1920, 0));

    let (x, y) = centred_in((0, 0, 2000, 1200), (0, 0, 1920, 1040), (0, 0, 1920, 1040));
    assert_eq!((x, y), (0, 0));
}
