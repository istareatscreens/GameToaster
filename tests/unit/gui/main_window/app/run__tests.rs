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

#[test]
fn the_main_window_has_a_taskbar_button_and_keeps_its_fixed_frameless_shape() {
    let icon = egui::IconData {
        rgba: vec![0; 4],
        width: 1,
        height: 1,
    };
    let viewport = main_viewport(true, icon.clone());
    assert_eq!(viewport.taskbar, Some(true));
    assert_eq!(viewport.visible, Some(true));
    assert_eq!(viewport.title.as_deref(), Some("GameToaster"));
    assert_eq!(viewport.app_id.as_deref(), Some("gametoaster-launcher"));
    assert_eq!(viewport.inner_size, Some(egui::vec2(1080.0, 800.0)));
    assert_eq!(viewport.min_inner_size, Some(egui::vec2(1080.0, 800.0)));
    assert_eq!(viewport.max_inner_size, Some(egui::vec2(1080.0, 800.0)));
    assert_eq!(viewport.decorations, Some(false));
    assert_eq!(viewport.resizable, Some(false));
    assert!(viewport.icon.is_some());

    let hidden = main_viewport(false, icon);
    assert_eq!(hidden.visible, Some(false));
    assert_eq!(hidden.taskbar, Some(true));
}
