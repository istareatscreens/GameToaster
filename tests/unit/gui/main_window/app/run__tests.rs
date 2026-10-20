use super::*;

fn config(enabled: bool, launch_on_start: bool, controller_only_start: bool) -> LauncherConfig {
    let mut config = LauncherConfig::default();
    let screen = &mut config.settings.controller_screen;
    screen.enabled = enabled;
    screen.launch_on_start = launch_on_start;
    screen.controller_only_start = controller_only_start;
    config
}

#[test]
fn a_plain_start_shows_the_main_window_and_the_tray_and_a_controller_only_start_hide_it() {
    assert!(main_window_starts_visible(
        false,
        &config(false, false, false),
        false
    ));
    assert!(!main_window_starts_visible(
        true,
        &config(false, false, false),
        false
    ));
    assert!(!main_window_starts_visible(
        false,
        &config(true, true, true),
        false
    ));
}

#[test]
fn a_disabled_controller_screen_never_hides_the_main_window() {
    assert!(main_window_starts_visible(
        false,
        &config(false, true, true),
        false
    ));
}

#[test]
fn a_pending_notice_opens_the_main_window_however_the_launcher_starts() {
    assert!(main_window_starts_visible(
        true,
        &config(false, false, false),
        true
    ));
    assert!(main_window_starts_visible(
        false,
        &config(true, true, true),
        true
    ));
    assert!(main_window_starts_visible(
        true,
        &config(true, true, true),
        true
    ));
}
