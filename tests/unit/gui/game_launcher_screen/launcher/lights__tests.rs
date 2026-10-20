use std::time::{Duration, Instant};

use super::super::tests::{game, test_launcher};
use super::*;
use crate::game_launcher_screen::{default_controller_config, ControllerLaunchSource};
use crate::launcher_gui::model::{GameId, LauncherConfig};

fn lit_config(ports: &str) -> LauncherConfig {
    let mut config = LauncherConfig::default();
    config.games.push(game(1, "Lights fixture"));
    let screen = &mut config.settings.controller_screen;
    screen.enabled = true;
    screen.controller = default_controller_config("adx-io4").unwrap();
    screen
        .controller
        .options
        .insert("turn_lights_off_on_game_close".into(), "true".into());
    screen
        .controller
        .options
        .insert("led_ports".into(), ports.into());
    config
}

fn eventually(mut done: impl FnMut() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn every_outcome_names_the_lights_the_ports_and_how_it_ended() {
    let fields = |outcome| outcome_fields("led-board", "COM21", &outcome);
    assert_eq!(
        fields(LightsOutcome::TurnedOff { attempts: 2 }),
        [
            ("lights", "led-board".to_owned()),
            ("ports", "COM21".to_owned()),
            ("outcome", "turned-off".to_owned()),
            ("attempts", "2".to_owned()),
        ]
    );
    assert_eq!(
        fields(LightsOutcome::Cancelled)[2],
        ("outcome", "cancelled-by-launch".to_owned())
    );
    assert_eq!(
        fields(LightsOutcome::GaveUp { attempts: 61 })[2..],
        [
            ("outcome", "port-still-in-use".to_owned()),
            ("attempts", "61".to_owned()),
        ]
    );
    assert_eq!(
        fields(LightsOutcome::Failed("no port".into()))[2..],
        [
            ("outcome", "failed".to_owned()),
            ("error", "no port".to_owned())
        ]
    );
}

#[test]
fn nothing_runs_unless_the_screen_the_driver_and_the_switch_all_allow_it() {
    let config = lit_config("COM255");

    let mut disabled = config.clone();
    disabled.settings.controller_screen.enabled = false;
    let mut launcher = test_launcher(&disabled, "lights-disabled");
    launcher.turn_lights_off_after_game(&disabled.settings.controller_screen);
    assert!(!launcher.lights_off.is_running(), "the screen is disabled");

    let mut switched_off = config.clone();
    switched_off
        .settings
        .controller_screen
        .controller
        .options
        .insert("turn_lights_off_on_game_close".into(), "false".into());
    let mut launcher = test_launcher(&switched_off, "lights-switched-off");
    launcher.turn_lights_off_after_game(&switched_off.settings.controller_screen);
    assert!(!launcher.lights_off.is_running(), "the switch is off");
}

#[test]
fn an_observed_exit_starts_the_job_and_its_outcome_reaches_the_diagnostics_log() {
    let config = lit_config("COM255");
    let mut launcher = test_launcher(&config, "lights-outcome");
    let log = launcher.launcher_dir.join("GameToaster.log");
    let before = std::fs::read_to_string(&log).unwrap_or_default().len();

    launcher.turn_lights_off_after_game(&config.settings.controller_screen);

    assert!(launcher.lights_off.is_running(), "the job started");
    assert!(
        eventually(|| {
            std::fs::read_to_string(&log).unwrap_or_default()[before..]
                .contains("operation=controller-lights-off")
        }),
        "no lights-off event was logged"
    );
    let logged = std::fs::read_to_string(&log).unwrap();
    let event = logged[before..]
        .lines()
        .find(|line| line.contains("operation=controller-lights-off"))
        .unwrap();
    assert!(event.contains("lights=\"led-board\""), "{event}");
    assert!(event.contains("ports=\"COM255\""), "{event}");
    assert!(event.contains("outcome=\"failed\""), "{event}");
}

#[test]
fn starting_a_launch_cancels_a_job_that_is_still_waiting() {
    let config = lit_config("COM255");
    let mut launcher = test_launcher(&config, "lights-cancel");
    let log = launcher.launcher_dir.join("GameToaster.log");
    let before = std::fs::read_to_string(&log).unwrap_or_default().len();
    launcher.turn_lights_off_after_game(&config.settings.controller_screen);
    assert!(launcher.lights_off.is_running());

    launcher
        .prepare_game_handoff(GameId(1), ControllerLaunchSource::MainGui, true, false)
        .unwrap();

    assert!(
        eventually(|| !launcher.lights_off.is_running()),
        "the job kept running after the launch started"
    );
    assert!(
        eventually(
            || std::fs::read_to_string(&log).unwrap_or_default()[before..]
                .contains("outcome=\"cancelled-by-launch\"")
        ),
        "the cancel was not logged"
    );
}
