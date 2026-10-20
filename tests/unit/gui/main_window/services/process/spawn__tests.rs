use super::*;

#[test]
fn every_launch_starts_suspended_with_its_console_flags() {
    let fixed = CREATE_SUSPENDED.0 | CREATE_UNICODE_ENVIRONMENT.0;
    for window in [
        ConsoleWindow::Graphical,
        ConsoleWindow::Windowless,
        ConsoleWindow::Hidden,
        ConsoleWindow::Shown,
    ] {
        let flags = creation_flags(window).0;
        assert_eq!(flags & fixed, fixed, "{window:?}");
        assert_eq!(flags & !fixed, window.creation_flags().0, "{window:?}");
    }
}
