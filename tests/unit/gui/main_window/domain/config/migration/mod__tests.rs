use std::collections::BTreeSet;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use super::super::save_atomic;
use super::fixtures::{self, as_lf, as_written};
use super::*;
use crate::game_launcher_screen::{
    ControllerConfig, LauncherXAction, MonitorCalibration, OrientationProfile,
};
use crate::launcher_gui::model::{CloseMode, GameId, PluginUpdatePolicy};

fn driver(driver_id: &str, options: &[(&str, &str)]) -> ControllerConfig {
    let mut config = crate::game_launcher_screen::default_controller_config(driver_id).unwrap();
    for (key, value) in options {
        config.options.insert((*key).into(), (*value).into());
    }
    config
}

const PREVIOUS: &str = "launcherConfig.ini";
const MONITOR_ID: &str =
    r"\\?\DISPLAY#ABC1234#5&1a2b3c4d&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}";

const SHARE_READ: u32 = 1;

fn entries(path: &Path) -> BTreeSet<(String, String, Option<String>)> {
    read_document(path)
        .unwrap()
        .get_map_ref()
        .iter()
        .flat_map(|(section, values)| {
            values
                .iter()
                .map(move |(key, value)| (section.clone(), key.clone(), value.clone()))
        })
        .collect()
}

fn written_entries(config: &LauncherConfig) -> BTreeSet<(String, String, Option<String>)> {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join(FILE_NAME);
    save_new(&path, config).unwrap();
    entries(&path)
}

fn names(folder: &Path) -> BTreeSet<String> {
    fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn only(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn is_dated_backup(name: &str, prefix: &str) -> bool {
    let Some(stamp) = name
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('.'))
        .and_then(|rest| rest.strip_suffix(".bak"))
    else {
        return false;
    };
    stamp.len() == 20
        && stamp.bytes().enumerate().all(|(index, byte)| match index {
            8 => byte == b'T',
            15 => byte == b'.',
            19 => byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

fn replaced(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "{from}");
    text.replace(from, to)
}

fn reset_reason(original: &[u8]) -> ResetReason {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    fs::write(&path, original).unwrap();
    let opened = open(state.path()).unwrap();
    let Some(Notice::Reset { reason, kept }) = opened.notice else {
        panic!("{:?}", opened.notice);
    };
    assert_eq!(kept.parent(), Some(state.path()));
    let name = kept.file_name().unwrap().to_string_lossy().into_owned();
    assert!(is_dated_backup(&name, FILE_NAME), "{name}");
    assert_eq!(fs::read(&kept).unwrap(), original);
    assert_eq!(opened.config, LauncherConfig::default());
    assert_eq!(entries(&path), written_entries(&LauncherConfig::default()));
    reason
}

#[test]
fn an_empty_folder_starts_with_the_defaults_and_writes_nothing() {
    let state = tempfile::tempdir().unwrap();
    let opened = open(state.path()).unwrap();
    assert_eq!(opened.config, LauncherConfig::default());
    assert_eq!(opened.notice, None);
    assert!(names(state.path()).is_empty());
}

#[test]
fn the_previous_launchers_file_becomes_a_schema_3_file_and_stays_untouched() {
    for text in [as_lf(fixtures::V1_FULL), as_written(fixtures::V1_FULL)] {
        let state = tempfile::tempdir().unwrap();
        let previous = state.path().join(PREVIOUS);
        fs::write(&previous, &text).unwrap();

        let opened = open(state.path()).unwrap();

        assert_eq!(
            opened.notice,
            Some(Notice::Upgraded {
                kept: previous.clone(),
                plugin_needed: vec!["日本語 ゲーム".into(), "Second Prepared".into()],
            })
        );
        assert_eq!(fs::read_to_string(&previous).unwrap(), text);
        assert_eq!(names(state.path()), only(&[FILE_NAME, PREVIOUS]));
        let current = state.path().join(FILE_NAME);
        assert_eq!(entries(&current), written_entries(&opened.config));

        let settings = &opened.config.settings;
        assert!(!settings.auto_hide_on_launch);
        assert!(settings.show_command_windows);
        assert!(settings.launch_default_on_open);
        assert!(settings.diagnostics_logging);
        assert!(settings.block_exit_on_credit);
        assert!(settings.pause_on_failure);
        assert_eq!(settings.language, "ja");
        assert_eq!(settings.adapter, Some(1));
        assert_eq!(
            settings.launcher_hotkey,
            Some("Ctrl+Alt+J".parse().unwrap())
        );
        assert_eq!(
            settings.close_game_hotkey,
            Some("Ctrl+Alt+K".parse().unwrap())
        );
        let screen = &settings.controller_screen;
        assert!(screen.enabled && screen.launch_on_start && screen.controller_only_start);
        assert!(screen.reopen_after_game_close && screen.open_on_os_startup);
        assert!(!screen.ba1_enabled && screen.ba2_enabled);
        assert!(screen.keep_controller_alive_when_minimized);
        assert_eq!(screen.launcher_x_action, LauncherXAction::ExitLauncher);
        assert_eq!(screen.monitor_adapter, Some(2));
        assert_eq!(
            screen.background_image,
            Some(PathBuf::from(r"C:\Art #2\background.png"))
        );
        assert_eq!(screen.orientation, OrientationProfile::ProductionLandscape);
        assert_eq!(
            screen.calibration,
            Some(MonitorCalibration {
                monitor_id: MONITOR_ID.into(),
                mode_width_px: 1920,
                mode_height_px: 1080,
                rotation_degrees: 90,
                profile: OrientationProfile::CabinetPortrait,
                diameter_px: 1000,
            })
        );
        assert_eq!(
            screen.controller,
            driver("adx-hid", &[("vid", "2E3C"), ("pid", "5750")])
        );

        assert_eq!(opened.config.default_game, Some(GameId(3)));
        assert_eq!(opened.config.next_game_id, 8);
        let games: Vec<(u64, &str, &Path)> = opened
            .config
            .games
            .iter()
            .map(|game| {
                (
                    game.id.0,
                    game.label.as_str(),
                    game.selected_target.as_path(),
                )
            })
            .collect();
        assert_eq!(
            games,
            [
                (3, "日本語 ゲーム", Path::new(r"C:\遊戲\example.exe")),
                (5, "Script Game", Path::new(r"D:\Games\Script\start.bat")),
                (
                    7,
                    "Second Prepared",
                    Path::new(r"D:\Games\Second\second.exe")
                ),
            ]
        );
        let game = &opened.config.games[0];
        assert_eq!(game.plugin, None);
        assert_eq!(
            game.controller_image,
            Some(PathBuf::from("launcher_assets/game_images/3.png"))
        );
        assert_eq!(game.hotkey, Some("Ctrl+Shift+F5".parse().unwrap()));
        assert_eq!(game.watch_exe.as_deref(), Some("example.exe"));
        assert_eq!(game.close_mode, CloseMode::Script);
        assert_eq!(game.close_script, Some(PathBuf::from(r"C:\遊戲\close.bat")));
        assert_eq!(game.close_hotkey, Some("Ctrl+Shift+F6".parse().unwrap()));
        assert_eq!(
            game.config_targets,
            [
                PathBuf::from(r"C:\遊戲\settings.ini"),
                PathBuf::from(r"C:\遊戲\tool.exe"),
            ]
        );
    }
}

#[test]
fn the_previous_launchers_jvs_controller_keeps_its_player_and_endpoint() {
    let state = tempfile::tempdir().unwrap();
    fs::write(state.path().join(PREVIOUS), as_written(fixtures::V1_JVS)).unwrap();

    let opened = open(state.path()).unwrap();

    let settings = &opened.config.settings;
    assert_eq!(
        settings.controller_screen.controller,
        driver("jvs", &[("player", "p2"), ("endpoint", "COM4")])
    );
    assert!(settings.controller_screen.enabled);
    assert_eq!(
        settings.launcher_hotkey, None,
        "an empty hotkey stays unbound"
    );
    assert_eq!(settings.close_game_hotkey, None);
    assert_eq!(settings.adapter, None, "a bare key has no value");
    assert_eq!(settings.controller_screen.monitor_adapter, None);
    assert_eq!(settings.controller_screen.background_image, None);
    assert_eq!(settings.controller_screen.calibration, None);
    assert_eq!(opened.config.default_game, None);
    assert_eq!(opened.config.games.len(), 1);
    assert_eq!(
        opened.config.games[0].selected_target,
        PathBuf::from(r"C:\Games\Only\only.exe")
    );
    assert!(
        matches!(&opened.notice, Some(Notice::Upgraded { plugin_needed, .. }) if plugin_needed.is_empty()),
        "{:?}",
        opened.notice
    );
}

#[test]
fn shipped_seed_files_and_a_default_file_upgrade_to_the_defaults() {
    for (name, text) in [
        ("defaults", fixtures::V1_DEFAULT),
        ("seed", fixtures::V1_SEED),
        ("release seed", fixtures::V1_SEED_RELEASE),
    ] {
        for text in [as_lf(text), as_written(text)] {
            let state = tempfile::tempdir().unwrap();
            let previous = state.path().join(PREVIOUS);
            fs::write(&previous, &text).unwrap();
            let opened = open(state.path()).unwrap();
            assert_eq!(opened.config, LauncherConfig::default(), "{name}");
            assert_eq!(
                opened.notice,
                Some(Notice::Upgraded {
                    kept: previous,
                    plugin_needed: Vec::new(),
                }),
                "{name}"
            );
        }
    }
}

#[test]
fn a_schema_2_file_is_upgraded_after_a_dated_backup() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    let original = as_written(fixtures::V2_FULL);
    fs::write(&path, &original).unwrap();

    let opened = open(state.path()).unwrap();

    let Some(Notice::Upgraded {
        kept,
        plugin_needed,
    }) = &opened.notice
    else {
        panic!("{:?}", opened.notice);
    };
    assert!(plugin_needed.is_empty());
    assert_eq!(kept.parent(), Some(state.path()));
    let name = kept.file_name().unwrap().to_string_lossy().into_owned();
    assert!(is_dated_backup(&name, "GameToaster.ini.schema-2"), "{name}");
    assert_eq!(fs::read_to_string(kept).unwrap(), original);
    assert_eq!(names(state.path()), only(&[FILE_NAME, &name]));
    assert_eq!(entries(&path), written_entries(&opened.config));

    let settings = &opened.config.settings;
    assert_eq!(settings.language, "zh-Hant");
    let screen = &settings.controller_screen;
    assert_eq!(
        screen.controller,
        driver("adx-hid", &[("vid", "2E3C"), ("pid", "5750")])
    );
    assert!(screen.ba1_enabled && !screen.ba2_enabled && !screen.controller_only_start);
    assert_eq!(screen.orientation, OrientationProfile::CabinetPortrait);
    assert_eq!(
        screen
            .calibration
            .as_ref()
            .map(|calibration| calibration.monitor_id.as_str()),
        Some(MONITOR_ID)
    );
    let game = &opened.config.games[0];
    assert_eq!(game.id, GameId(3));
    let plugin = game.plugin.as_ref().expect("the plugin assignment is kept");
    assert_eq!(plugin.plugin_id, "example.plugin");
    assert_eq!(plugin.revision.version.to_string(), "1.2.3");
    assert_eq!(plugin.update_policy, PluginUpdatePolicy::Pinned);
    assert!(plugin.copy_zero_matches_to_game_folder);
    assert!(!plugin.search_directory_destinations && !plugin.allow_prerelease);
    assert_eq!(plugin.assignment_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(
        plugin.resolved_launcher,
        Some(PathBuf::from(r"C:\遊戲\launcher.exe"))
    );
    assert_eq!(plugin.resolved_watch_target, None);
    assert_eq!(plugin.grants.len(), 1);
    assert_eq!(plugin.grants[0].destination_root, PathBuf::from(r"C:\遊戲"));
    assert_eq!(
        plugin.ledger_reference,
        PathBuf::from(r"C:\State\plugins\ledgers\3.toml")
    );
    assert_eq!(
        opened.config.games[1].selected_target,
        PathBuf::from(r"D:\Games\Script\start.bat")
    );
}

#[test]
fn a_schema_2_controller_becomes_its_schema_3_driver() {
    for (text, expected) in [
        (
            fixtures::V2_JVS,
            driver("jvs", &[("player", "p1"), ("endpoint", r"\\.\mxjvs")]),
        ),
        (
            fixtures::V2_IO4,
            driver("adx-io4", &[("vid", "0CA3"), ("pid", "0022")]),
        ),
    ] {
        let state = tempfile::tempdir().unwrap();
        fs::write(state.path().join(FILE_NAME), as_written(text)).unwrap();
        let opened = open(state.path()).unwrap();
        assert_eq!(
            opened.config.settings.controller_screen.controller,
            expected
        );
    }
}

#[test]
fn a_second_start_after_an_upgrade_changes_nothing() {
    for (file, text) in [
        (PREVIOUS, fixtures::V1_FULL),
        (FILE_NAME, fixtures::V2_FULL),
    ] {
        let state = tempfile::tempdir().unwrap();
        fs::write(state.path().join(file), as_written(text)).unwrap();
        let first = open(state.path()).unwrap();
        let files = names(state.path());
        let written = fs::read(state.path().join(FILE_NAME)).unwrap();

        let second = open(state.path()).unwrap();

        assert_eq!(second.config, first.config, "{file}");
        assert_eq!(second.notice, None, "{file}");
        assert_eq!(names(state.path()), files, "{file}");
        assert_eq!(fs::read(state.path().join(FILE_NAME)).unwrap(), written);
    }
}

#[test]
fn a_current_file_opens_without_a_notice_or_a_backup() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    let mut config = LauncherConfig::default();
    config.settings.language = "ja".into();
    save_atomic(&path, &config).unwrap();
    let before = fs::read(&path).unwrap();

    let opened = open(state.path()).unwrap();

    assert_eq!(opened.config, config);
    assert_eq!(opened.notice, None);
    assert_eq!(names(state.path()), only(&[FILE_NAME]));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn the_current_file_wins_over_the_previous_launchers_file() {
    let state = tempfile::tempdir().unwrap();
    let mut config = LauncherConfig::default();
    config.settings.language = "zh-Hans".into();
    save_atomic(&state.path().join(FILE_NAME), &config).unwrap();
    let previous = state.path().join(PREVIOUS);
    fs::write(&previous, as_written(fixtures::V1_FULL)).unwrap();

    let opened = open(state.path()).unwrap();

    assert_eq!(opened.config, config);
    assert_eq!(opened.notice, None);
    assert_eq!(
        fs::read_to_string(&previous).unwrap(),
        as_written(fixtures::V1_FULL)
    );
}

#[test]
fn a_file_without_a_version_is_kept_and_replaced_by_the_defaults() {
    assert_eq!(
        reset_reason(b"[launcher]\r\nnext_game_id=2\r\n"),
        ResetReason::UnknownVersion
    );
    assert_eq!(
        reset_reason(b"[launcher]\r\nschema_version=three\r\n"),
        ResetReason::UnknownVersion
    );
}

#[test]
fn a_newer_file_is_kept_and_replaced_by_the_defaults() {
    assert_eq!(
        reset_reason(b"[launcher]\r\nschema_version=4\r\n"),
        ResetReason::Newer(4)
    );
}

#[test]
fn an_unreadable_file_is_kept_and_replaced_by_the_defaults() {
    for original in [
        &b"[launcher]\r\nschema_version=3\r\nlanguage=\xff\r\n"[..],
        b"[launcher\r\nschema_version=3\r\n",
    ] {
        let reason = reset_reason(original);
        assert!(matches!(reason, ResetReason::Unreadable(_)), "{reason:?}");
    }
}

#[test]
fn a_file_a_step_cannot_upgrade_is_kept_and_replaced_by_the_defaults() {
    let unknown_adapter = replaced(
        &as_written(fixtures::V2_IO4),
        "controller_adapter=adx-io4",
        "controller_adapter=keyboard",
    );
    assert_eq!(
        reset_reason(unknown_adapter.as_bytes()),
        ResetReason::NotUpgradable(
            "upgrading version 2: unknown controller adapter 'keyboard'".into()
        )
    );
}

#[test]
fn an_upgrade_the_current_schema_refuses_is_kept_and_replaced_by_the_defaults() {
    let too_large = replaced(
        &as_written(fixtures::V2_IO4),
        "next_game_id=2",
        "next_game_id=9223372036854775808",
    );
    let reason = reset_reason(too_large.as_bytes());
    assert!(
        matches!(&reason, ResetReason::NotUpgradable(error) if error.contains("next_game_id")),
        "{reason:?}"
    );
}

#[test]
fn a_previous_launchers_file_that_cannot_be_upgraded_stays_where_it_is() {
    let state = tempfile::tempdir().unwrap();
    let previous = state.path().join(PREVIOUS);
    let original = replaced(&as_written(fixtures::V1_JVS), "kind=other", "kind=retro");
    fs::write(&previous, &original).unwrap();

    let opened = open(state.path()).unwrap();

    assert_eq!(
        opened.notice,
        Some(Notice::Reset {
            reason: ResetReason::NotUpgradable(
                "upgrading version 1: unknown game kind 'retro' in [game.1]".into()
            ),
            kept: previous.clone(),
        })
    );
    assert_eq!(fs::read_to_string(&previous).unwrap(), original);
    assert_eq!(opened.config, LauncherConfig::default());
    assert_eq!(
        entries(&state.path().join(FILE_NAME)),
        written_entries(&LauncherConfig::default())
    );
    assert_eq!(names(state.path()), only(&[FILE_NAME, PREVIOUS]));
}

#[test]
fn a_byte_order_mark_does_not_lose_the_previous_launchers_settings() {
    let state = tempfile::tempdir().unwrap();
    fs::write(
        state.path().join(PREVIOUS),
        format!("\u{feff}{}", as_written(fixtures::V1_FULL)),
    )
    .unwrap();
    let opened = open(state.path()).unwrap();
    assert_eq!(opened.config.settings.language, "ja");
    assert_eq!(opened.config.games.len(), 3);
}

#[test]
fn a_byte_order_mark_does_not_lose_a_schema_2_files_settings() {
    let state = tempfile::tempdir().unwrap();
    fs::write(
        state.path().join(FILE_NAME),
        format!("\u{feff}{}", as_written(fixtures::V2_FULL)),
    )
    .unwrap();
    let opened = open(state.path()).unwrap();
    assert!(matches!(opened.notice, Some(Notice::Upgraded { .. })));
    assert_eq!(opened.config.settings.language, "zh-Hant");
    assert_eq!(opened.config.games.len(), 2);
}

#[test]
fn a_schema_2_file_under_the_name_its_writer_gave_it_is_upgraded() {
    let state = tempfile::tempdir().unwrap();
    let original = as_written(fixtures::V2_FULL);
    fs::write(state.path().join("gameToaster.ini"), &original).unwrap();

    let opened = open(state.path()).unwrap();

    let Some(Notice::Upgraded { kept, .. }) = &opened.notice else {
        panic!("{:?}", opened.notice);
    };
    assert_eq!(fs::read_to_string(kept).unwrap(), original);
    assert_eq!(opened.config.settings.language, "zh-Hant");
    let lowered: BTreeSet<String> = names(state.path())
        .into_iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    assert_eq!(lowered.len(), 2);
    assert!(lowered.contains("gametoaster.ini"));
    assert_eq!(
        entries(&state.path().join(FILE_NAME)),
        written_entries(&opened.config)
    );
}

#[test]
fn an_unreadable_previous_launchers_file_stays_where_it_is() {
    let state = tempfile::tempdir().unwrap();
    let previous = state.path().join(PREVIOUS);
    let original = b"[launcher]\r\nlanguage=\xff\xfe\r\n".to_vec();
    fs::write(&previous, &original).unwrap();

    let opened = open(state.path()).unwrap();

    let Some(Notice::Reset {
        reason: ResetReason::Unreadable(detail),
        kept,
    }) = &opened.notice
    else {
        panic!("{:?}", opened.notice);
    };
    assert!(detail.contains("failed to read"), "{detail}");
    assert_eq!(kept, &previous);
    assert_eq!(fs::read(&previous).unwrap(), original);
    assert_eq!(opened.config, LauncherConfig::default());
    assert_eq!(names(state.path()), only(&[FILE_NAME, PREVIOUS]));
}

#[test]
fn a_current_file_with_an_invalid_value_stops_the_start_unchanged() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    let original = "[launcher]\r\nschema_version=3\r\nauto_hide_on_launch=perhaps\r\n";
    fs::write(&path, original).unwrap();

    let error = open(state.path()).err().expect("the start stops");

    assert!(error.contains("auto_hide_on_launch"), "{error}");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(names(state.path()), only(&[FILE_NAME]));
}

#[test]
fn a_file_that_cannot_be_backed_up_stops_the_start_unchanged() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    let original = b"[launcher]\r\nschema_version=9\r\n";
    fs::write(&path, original).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    let error = open(state.path()).err().expect("the start stops");

    drop(held);
    assert!(error.contains("failed to back up"), "{error}");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(names(state.path()), only(&[FILE_NAME]));
}

#[test]
fn an_upgrade_that_cannot_be_written_stops_the_start_after_the_backup() {
    let state = tempfile::tempdir().unwrap();
    let path = state.path().join(FILE_NAME);
    let original = as_written(fixtures::V2_IO4);
    fs::write(&path, &original).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ)
        .open(&path)
        .unwrap();

    let error = open(state.path()).err().expect("the start stops");

    drop(held);
    assert!(error.contains("failed to save configuration"), "{error}");
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    let backups: Vec<String> = names(state.path())
        .into_iter()
        .filter(|name| name != FILE_NAME)
        .collect();
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert!(
        is_dated_backup(&backups[0], "GameToaster.ini.schema-2"),
        "{backups:?}"
    );
}

#[test]
fn an_upgrade_of_the_previous_launchers_file_that_cannot_be_written_stops_the_start() {
    let state = tempfile::tempdir().unwrap();
    let previous = state.path().join(PREVIOUS);
    fs::write(&previous, as_written(fixtures::V1_FULL)).unwrap();
    fs::create_dir(state.path().join(FILE_NAME)).unwrap();

    let error = open(state.path()).err().expect("the start stops");

    assert!(error.contains("failed to save configuration"), "{error}");
    assert_eq!(
        fs::read_to_string(&previous).unwrap(),
        as_written(fixtures::V1_FULL)
    );
}
