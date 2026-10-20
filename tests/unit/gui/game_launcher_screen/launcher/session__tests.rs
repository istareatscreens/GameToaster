use super::super::monitor_return::MonitorReturnState;
use super::super::tests::{absent_display_fixture, game, monitor_return_fixture, test_launcher};
use crate::game_launcher_screen::{
    default_controller_config, ControllerLaunchSource, OrientationProfile,
};
use crate::gui::game_launcher_screen::{display, surface};
use crate::launcher_gui::model::{GameId, LauncherConfig};

#[test]
fn open_close_is_calibration_gated_and_restore_tolerates_missing_controller() {
    let monitors = display::monitor::enumerate().unwrap();
    let Some(monitor) = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())
    else {
        return;
    };
    let mut config = LauncherConfig::default();
    config.games.push(game(1, "One"));

    config.settings.controller_screen.orientation = OrientationProfile::ProductionLandscape;
    let mut launcher = test_launcher(&config, "open-close");
    assert!(launcher.open(&config.settings.controller_screen).is_err());

    let profile = display::geometry::resolve_profile(
        config.settings.controller_screen.orientation,
        monitor.bounds,
    );
    let screen = &mut config.settings.controller_screen;
    screen.controller = default_controller_config("jvs").unwrap();
    screen.monitor_adapter = Some(monitor.index);
    screen.calibration = Some(
        surface::circle::calibration::build(
            monitor.id.key(),
            (monitor.bounds.width, monitor.bounds.height),
            monitor.rotation_degrees,
            profile,
            monitor.bounds.width.min(monitor.bounds.height),
        )
        .unwrap(),
    );
    assert!(launcher.open(screen).is_ok());
    assert!(launcher.is_open());
    assert_eq!(launcher.viewport_resources(), (true, true, 0));
    assert!(launcher
        .prepare_game_handoff(GameId(1), ControllerLaunchSource::Controller, true, false)
        .unwrap());
    launcher.launch_succeeded().unwrap();
    assert!(launcher.game_exited());
    assert!(launcher.open(screen).is_ok());
    launcher.restore_complete();
    assert!(launcher.is_open());
    launcher.close().unwrap();
    assert!(!launcher.is_open());

    assert_eq!(launcher.viewport_resources(), (false, true, 0));
}

#[test]
fn an_unregistered_layout_still_reports_the_absent_display_first() {
    let _locale = crate::gui::common::harness::locale("en");
    let Some((config, mut screen)) = absent_display_fixture() else {
        return;
    };
    screen.launcher_layout = "unregistered-layout".into();
    let mut launcher = test_launcher(&config, "unregistered-layout");
    let error = launcher.open(&screen).unwrap_err();
    assert_eq!(
        error.shown,
        "the configured game launcher monitor is not active"
    );
    assert!(matches!(
        launcher.monitor_return,
        MonitorReturnState::Waiting(_)
    ));
}

#[test]
fn an_invalid_layout_id_is_refused_by_activation_after_the_display_resolves() {
    let _locale = crate::gui::common::harness::locale("en");
    let Some((config, mut screen)) = monitor_return_fixture() else {
        return;
    };
    screen.launcher_layout = "Bad".into();
    let mut launcher = test_launcher(&config, "invalid-layout-id");
    let error = launcher.open(&screen).unwrap_err();
    assert_eq!(
        error.shown,
        "launcher layout ID must use lowercase letters, digits, and hyphens"
    );
    assert!(!launcher.is_open());
}
