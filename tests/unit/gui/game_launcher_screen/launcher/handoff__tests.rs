use super::super::tests::{game, test_launcher};
use super::*;
use crate::game_launcher_screen::default_controller_config;
use crate::gui::game_launcher_screen::{display, surface};
use crate::launcher_gui::model::LauncherConfig;

#[test]
fn running_state_has_no_controller_resources_or_viewport_submission() {
    let monitors = display::monitor::enumerate().unwrap();
    let Some(monitor) = monitors
        .iter()
        .find(|monitor| monitor.primary)
        .or_else(|| monitors.first())
    else {
        return;
    };
    let mut config = LauncherConfig::default();
    config.games.push(game(1, "Invariant fixture"));
    let screen = &mut config.settings.controller_screen;
    screen.controller = default_controller_config("jvs").unwrap();
    screen.monitor_adapter = Some(monitor.index);
    let profile = display::geometry::resolve_profile(screen.orientation, monitor.bounds);
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
    let screen = config.settings.controller_screen.clone();
    let mut launcher = test_launcher(&config, "running-invariants");
    launcher.open(&screen).unwrap();
    launcher
        .prepare_game_handoff(GameId(1), ControllerLaunchSource::MainGui, true, false)
        .unwrap();
    launcher.launch_succeeded().unwrap();
    assert_eq!(
        launcher.lifecycle.state(),
        lifecycle::coordinator::State::Running
    );

    assert_eq!(launcher.viewport_resources(), (false, true, 0));
    assert_eq!(launcher.resource_counts(), (0, 0));
}
