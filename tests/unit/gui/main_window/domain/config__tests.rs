use std::fs;

use super::*;

fn test_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gametoaster-launcher-config-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn load(path: &Path) -> Result<LauncherConfig, String> {
    parse_current(&read_document(path)?)
}

fn sample() -> LauncherConfig {
    LauncherConfig {
        settings: LauncherSettings {
            auto_hide_on_launch: false,
            show_command_windows: true,
            launch_default_on_open: true,
            diagnostics_logging: true,
            block_exit_on_credit: true,
            pause_on_failure: true,
            language: "ja".into(),
            adapter: Some(1),
            launcher_hotkey: Some("Ctrl+Alt+J".parse().unwrap()),
            close_game_hotkey: Some("Ctrl+Alt+K".parse().unwrap()),
            controller_screen: ControllerScreenConfig {
                enabled: true,
                launch_on_start: true,
                reopen_after_game_close: true,
                reopen_on_monitor_return: true,
                controller_only_start: true,
                open_on_os_startup: true,
                ba1_enabled: false,
                ba2_enabled: true,
                keep_controller_alive_when_minimized: true,
                launcher_x_action: LauncherXAction::ExitLauncher,
                monitor_adapter: Some(2),
                background_image: Some(std::path::PathBuf::from("C:\\art\\bg.png")),
                launcher_layout: "washing-machine".into(),
                orientation: OrientationProfile::ProductionLandscape,
                calibration: Some(MonitorCalibration {
                    monitor_id: "DISPLAY\\CABINET|serial-42".into(),
                    mode_width_px: 1920,
                    mode_height_px: 1080,
                    rotation_degrees: 0,
                    profile: OrientationProfile::ProductionLandscape,
                    diameter_px: 1000,
                }),

                controller: crate::game_launcher_screen::default_controller_config("adx-hid")
                    .unwrap(),
            },
        },
        default_game: Some(GameId(8)),
        next_game_id: 9,
        games: vec![GameEntry {
            id: GameId(8),
            label: "日本語 Test".into(),
            controller_image: Some(PathBuf::from("launcher_assets/game_images/8.png")),
            selected_target: PathBuf::from(r"C:\遊戲\game.exe"),
            plugin: Some(PluginAssignment {
                plugin_id: "example.plugin".into(),
                revision: PluginRevision {
                    version: "1.2.3".parse().unwrap(),
                    sha256: "a".repeat(64),
                },
                update_policy: PluginUpdatePolicy::Pinned,
                allow_prerelease: false,
                copy_zero_matches_to_game_folder: true,
                search_directory_destinations: true,
                assignment_id: "b".repeat(32),
                resolved_launcher: Some(PathBuf::from(r"C:\遊戲\plugin-launcher.exe")),
                resolved_watch_target: Some(PathBuf::from(r"C:\遊戲\game.exe")),
                grants: vec![ExternalGrant {
                    plugin_id: "example.plugin".into(),
                    game_id: 8,
                    revision_sha256: "a".repeat(64),
                    destination_root: PathBuf::from(r"D:\shared-config"),
                    root_identity: None,
                }],
                ledger_reference: PathBuf::from(
                    r"C:\state\assignments\8\bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\ledger.toml",
                ),
            }),
            hotkey: Some("Ctrl+Shift+F5".parse().unwrap()),
            watch_exe: Some("game.exe".into()),
            close_mode: CloseMode::Script,
            close_script: Some(PathBuf::from(r"C:\遊戲\close.bat")),
            close_hotkey: Some("Ctrl+Shift+F6".parse().unwrap()),
            config_targets: vec![
                PathBuf::from(r"C:\遊戲\extra.ini"),
                PathBuf::from(r"C:\遊戲\tool.exe"),
            ],
        }],
    }
}

#[test]
fn atomic_round_trip_preserves_unicode_and_settings() {
    let dir = test_dir("unicode");
    let path = dir.join(FILE_NAME);
    fs::write(
        &path,
        "[launcher]\nfuture_launcher_key=preserve-me\n[third_party]\nopaque=value\n",
    )
    .unwrap();
    let expected = sample();
    save_atomic(&path, &expected).unwrap();
    assert_eq!(load(&path).unwrap(), expected);
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("future_launcher_key=preserve-me"));
    assert!(saved.contains("[third_party]"));
    assert!(saved.contains("opaque=value"));
    assert!(
        fs::read_dir(&dir).unwrap().all(|entry| !entry
            .unwrap()
            .path()
            .to_string_lossy()
            .ends_with(".tmp")),
        "no temporary file is left behind"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn default_replacement_and_removal_are_consistent() {
    let mut config = sample();
    config.games.push(GameEntry {
        id: GameId(9),
        label: "Other".into(),
        controller_image: None,
        selected_target: PathBuf::from(r"D:\Games\other.exe"),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    });
    config.next_game_id = 10;
    config.set_default(Some(GameId(9))).unwrap();
    assert_eq!(config.default_game, Some(GameId(9)));
    config.games[1].label = "Relabeled".into();
    config.remove_game(GameId(9)).unwrap();
    assert_eq!(config.default_game, None);
    config.validate().unwrap();
}

#[test]
fn invalid_values_are_rejected_without_panicking() {
    let dir = test_dir("invalid");
    let path = dir.join(FILE_NAME);
    fs::write(
        &path,
        "[launcher]\nschema_version=3\nauto_hide_on_launch=perhaps\nnext_game_id=nope\n",
    )
    .unwrap();
    assert!(load(&path).unwrap_err().contains("auto_hide_on_launch"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn partial_plugin_assignment_is_rejected() {
    let dir = test_dir("partial-plugin");
    let path = dir.join(FILE_NAME);
    fs::write(
        &path,
        "[launcher]\nschema_version=3\nnext_game_id=2\n[game.1]\nlabel=Game\nselected_target=C:\\Games\\game.exe\nplugin_version=1.0.0\n",
    )
    .unwrap();
    let error = load(&path).unwrap_err();
    assert!(error.contains("missing plugin_id"), "{error}");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn malformed_controller_fields_are_rejected_with_field_context() {
    let _locale = crate::gui::common::harness::locale("en");
    let dir = test_dir("invalid-controller");
    let path = dir.join(FILE_NAME);
    fs::write(
        &path,
        "[launcher]\nschema_version=3\nnext_game_id=1\n[game_launcher_screen]\ncontroller_driver=adx-hid\ncontroller_option.vid=not-hex\ncontroller_option.pid=5750\n",
    )
    .unwrap();
    let error = load(&path).unwrap_err();
    assert!(
        error.contains("option 'vid'") && error.contains("hexadecimal"),
        "{error}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn generic_driver_switch_removes_stale_option_keys() {
    let dir = test_dir("generic-controller-keys");
    let path = dir.join(FILE_NAME);
    let mut config = sample();
    save_atomic(&path, &config).unwrap();
    config.settings.controller_screen.controller =
        crate::game_launcher_screen::default_controller_config("jvs").unwrap();
    save_atomic(&path, &config).unwrap();

    let saved = fs::read_to_string(&path).unwrap().to_ascii_lowercase();
    assert!(saved.contains("controller_driver=jvs"));
    assert!(saved.contains("controller_option.player=p1"));
    assert!(saved.contains("controller_option.endpoint=\\\\.\\mxjvs"));
    assert!(!saved.contains("controller_option.vid="));
    assert!(!saved.contains("controller_option.pid="));
    assert!(!saved.contains("jvs_hardware_validated"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_hidden_credit_block_setting_still_round_trips_through_the_config_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(FILE_NAME);
    let mut config = LauncherConfig::default();
    config.settings.block_exit_on_credit = true;
    save_atomic(&path, &config).unwrap();
    let loaded = load(&path).unwrap();
    assert!(loaded.settings.block_exit_on_credit);

    config.settings.block_exit_on_credit = false;
    save_atomic(&path, &config).unwrap();
    assert!(!load(&path).unwrap().settings.block_exit_on_credit);
}

#[test]
fn calibration_monitor_id_with_hash_characters_survives_load_and_round_trip() {
    let dir = test_dir("monitor-id-hash");
    let path = dir.join(FILE_NAME);
    let id = r"\\.\DISPLAY5|\\?\DISPLAY#GSM0001#5&ffbf381&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}|GSM|1|";
    let mut expected = sample();
    expected.settings.controller_screen.calibration = Some(MonitorCalibration {
        monitor_id: id.into(),
        mode_width_px: 2160,
        mode_height_px: 3840,
        rotation_degrees: 90,
        profile: OrientationProfile::CabinetPortrait,
        diameter_px: 2160,
    });
    save_atomic(&path, &expected).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(
        saved.contains(&format!("calibration_monitor_id={id}")),
        "{saved}"
    );

    let loaded = load(&path).unwrap();
    assert_eq!(
        loaded
            .settings
            .controller_screen
            .calibration
            .as_ref()
            .map(|calibration| calibration.monitor_id.as_str()),
        Some(id)
    );
    assert_eq!(loaded, expected);

    let mut labelled = expected.clone();
    labelled.games[0].label = "Game #1; special".into();
    save_atomic(&path, &labelled).unwrap();
    assert_eq!(load(&path).unwrap().games[0].label, "Game #1; special");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn game_ids_above_the_signed_64_bit_range_are_rejected() {
    let dir = test_dir("id-range");
    let path = dir.join(FILE_NAME);
    save_atomic(&path, &sample()).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    let next = saved
        .lines()
        .find(|line| line.starts_with("next_game_id"))
        .expect("the next id is written");
    fs::write(
        &path,
        saved.replace(next, "next_game_id=9223372036854775808"),
    )
    .unwrap();
    let error = load(&path).unwrap_err();
    assert!(
        error.contains("next_game_id") && error.contains("largest game id"),
        "{error}"
    );
    let section = saved
        .lines()
        .find(|line| line.starts_with("[game."))
        .expect("a game section is written");
    fs::write(&path, saved.replace(section, "[game.9223372036854775808]")).unwrap();
    let error = load(&path).unwrap_err();
    assert!(error.contains("largest game id"), "{error}");

    fs::write(
        &path,
        saved.replace(next, "next_game_id=9223372036854775807"),
    )
    .unwrap();
    assert_eq!(load(&path).unwrap().next_game_id, i64::MAX as u64);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_byte_order_mark_does_not_hide_the_first_section() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(FILE_NAME);
    fs::write(
        &path,
        "\u{feff}[launcher]\r\nschema_version=3\r\nauto_hide_on_launch=false\r\nlanguage=ja\r\n",
    )
    .unwrap();
    let loaded = load(&path).unwrap();
    assert!(!loaded.settings.auto_hide_on_launch);
    assert_eq!(loaded.settings.language, "ja");

    save_atomic(&path, &loaded).unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(!saved.contains('\u{feff}'), "{saved}");
    assert_eq!(load(&path).unwrap(), loaded);
}
