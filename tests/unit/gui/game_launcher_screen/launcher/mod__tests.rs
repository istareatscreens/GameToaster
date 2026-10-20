use super::*;
use crate::game_launcher_screen::{
    default_controller_config, ControllerScreenConfig, OrientationProfile,
};
use crate::gui::game_launcher_screen::{display, surface};
use crate::launcher_gui::model::{CloseMode, GameEntry, GameId};

pub(super) fn game(id: u64, label: &str) -> GameEntry {
    GameEntry {
        id: GameId(id),
        label: label.into(),
        controller_image: None,
        selected_target: format!(r"C:\games\{id}.exe").into(),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

pub(super) fn test_launcher(config: &LauncherConfig, name: &str) -> ControllerLauncher {
    let directory = std::env::temp_dir().join(format!(
        "gametoaster-controller-facade-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let log = crate::launcher_gui::logging::LauncherLog::open(&directory).unwrap();

    log.set_enabled(true);
    ControllerLauncher::new(directory, config, log, egui::Context::default())
}

pub(super) fn monitor_return_fixture() -> Option<(LauncherConfig, ControllerScreenConfig)> {
    let monitors = display::monitor::enumerate().unwrap();
    let monitor = monitors.first()?;
    let mut config = LauncherConfig::default();
    let screen = &mut config.settings.controller_screen;
    screen.enabled = true;
    screen.reopen_on_monitor_return = true;
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
    screen.controller = default_controller_config("jvs").unwrap();
    let screen = screen.clone();
    Some((config, screen))
}

pub(super) fn absent_display_fixture() -> Option<(LauncherConfig, ControllerScreenConfig)> {
    display::monitor::enumerate().ok()?;
    let mut config = LauncherConfig::default();
    let screen = &mut config.settings.controller_screen;
    screen.enabled = true;
    screen.reopen_on_monitor_return = true;
    screen.monitor_adapter = Some(99);
    screen.calibration = Some(
        surface::circle::calibration::build(
            r"\\.\DISPLAY99|\\?\DISPLAY#TEST0001#0&0&0&UID0#{00000000-0000-0000-0000-000000000000}|TST|1|"
                .to_string(),
            (2160, 3840),
            90,
            OrientationProfile::CabinetPortrait,
            2160,
        )
        .unwrap(),
    );
    screen.controller = default_controller_config("jvs").unwrap();
    let screen = screen.clone();
    Some((config, screen))
}

#[test]
fn committed_library_snapshots_apply_once_and_preserve_selection() {
    let mut config = LauncherConfig {
        games: vec![game(1, "One"), game(2, "Two"), game(3, "Three")],
        ..Default::default()
    };
    let mut launcher = test_launcher(&config, "library");
    launcher.games.select_next();
    assert_eq!(launcher.games.selected(), Some(GameId(2)));

    config.games[1].label = "Two renamed".into();
    config.games[1].controller_image = Some("launcher_assets/game_images/2.png".into());
    config.games.push(game(4, "Four"));
    launcher.library_changed(&config);
    assert_eq!(launcher.games.selected(), Some(GameId(2)));
    assert_eq!(launcher.games.visible_window(8).len(), 4);

    config.remove_game(GameId(2));
    launcher.library_changed(&config);
    assert_eq!(launcher.games.selected(), Some(GameId(3)));
    assert_eq!(
        launcher.games.visible_window(8),
        vec![GameId(1), GameId(3), GameId(4)]
    );
}
