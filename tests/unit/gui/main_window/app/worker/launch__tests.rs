use std::path::PathBuf;

use super::*;
use crate::launcher_gui::model::{
    GameEntry, GameId, PluginAssignment, PluginRevision, PluginUpdatePolicy,
};
use crate::plugin::lifecycle::RevisionPolicy;

fn game(policy: PluginUpdatePolicy, allow_prerelease: bool) -> GameEntry {
    GameEntry {
        id: GameId(7),
        label: "Game".into(),
        controller_image: None,
        selected_target: PathBuf::from(r"C:\games\game.exe"),
        plugin: Some(PluginAssignment {
            plugin_id: "org.example.plugin".into(),
            revision: PluginRevision {
                version: semver::Version::parse("1.2.0").unwrap(),
                sha256: "f".repeat(64),
            },
            update_policy: policy,
            allow_prerelease,
            copy_zero_matches_to_game_folder: true,
            search_directory_destinations: false,
            assignment_id: "0123456789abcdef0123456789abcdef".into(),
            resolved_launcher: None,
            resolved_watch_target: None,
            grants: Vec::new(),
            ledger_reference: PathBuf::from(r"C:\state\ledger.toml"),
        }),
        hotkey: None,
        watch_exe: None,
        pre_start_script: None,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

#[test]
fn a_launch_asks_for_exactly_the_pinned_revision_or_the_latest_stable_one() {
    let pinned = reconcile_request(&game(PluginUpdatePolicy::Pinned, true)).unwrap();
    assert_eq!(
        pinned.policy,
        RevisionPolicy::Pinned {
            version: semver::Version::parse("1.2.0").unwrap(),
            sha256: "f".repeat(64),
        },
        "a pinned game never leaves its revision, whatever its prerelease flag"
    );
    let tracking = reconcile_request(&game(PluginUpdatePolicy::TrackLatest, false)).unwrap();
    assert_eq!(
        tracking.policy,
        RevisionPolicy::TrackLatest {
            allow_prerelease: false
        }
    );
    assert_eq!(
        reconcile_request(&game(PluginUpdatePolicy::TrackLatest, true))
            .unwrap()
            .policy,
        RevisionPolicy::TrackLatest {
            allow_prerelease: true
        }
    );

    assert_eq!(tracking.assignment_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(tracking.plugin_id, "org.example.plugin");
    assert!(tracking.copy_zero_matches_to_game_folder);
    assert!(!tracking.search_directory_destinations);

    let mut plain = game(PluginUpdatePolicy::Pinned, false);
    plain.plugin = None;
    assert!(reconcile_request(&plain).is_err());
}

#[test]
fn the_files_a_launch_left_with_the_operator_reach_the_log_when_diagnostics_are_on() {
    let root = tempfile::tempdir().unwrap();
    let log = LauncherLog::open(root.path()).unwrap();
    let written = || std::fs::read_to_string(root.path().join("GameToaster.log")).unwrap();
    let kept = [
        PathBuf::from(r"C:\games\hook.dll"),
        PathBuf::from(r"C:\games\tools\helper.exe"),
    ];

    log_kept_files(&log, GameId(7), &kept);
    assert_eq!(written(), "", "diagnostics logging is off by default");

    log.set_enabled(true);
    log_kept_files(&log, GameId(7), &[]);
    assert_eq!(written(), "", "a launch that kept nothing names nothing");

    log_kept_files(&log, GameId(7), &kept);
    let lines: Vec<String> = written().lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    for (line, path) in lines.iter().zip([
        r#"path="C:\\games\\hook.dll""#,
        r#"path="C:\\games\\tools\\helper.exe""#,
    ]) {
        assert!(line.contains("operation=plugin-file-kept"), "{line}");
        assert!(line.contains(r#"game_id="7""#), "{line}");
        assert!(line.contains(path), "{line}");
    }
}

mod scripts {
    use std::time::{Duration, Instant};

    use super::super::super::fixtures::{config, game, released_handoff, Harness};
    use super::super::super::messages::{LaunchStage, WorkerResult};
    use crate::gui::common::harness::locale;
    use crate::launcher_gui::model::{PluginAssignment, PluginRevision, PluginUpdatePolicy};
    use crate::launcher_gui::runtime::LaunchCancel;

    #[test]
    fn the_pre_start_script_ends_before_the_game_starts() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
        entry.pre_start_script = Some(harness.script("start server.bat", "pre", &[], 0));

        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );

        assert!(
            matches!(result, WorkerResult::Launched { .. }),
            "{result:?}"
        );

        let posted = harness.posted();
        let shape: Vec<&str> = posted
            .iter()
            .map(|result| match result {
                WorkerResult::LaunchStage(LaunchStage::PreStartScript) => "pre-start",
                WorkerResult::ProgramStarted { .. } => "program",
                WorkerResult::LaunchStage(LaunchStage::Launching) => "launching",
                WorkerResult::HandoffRequired { .. } => "handoff",
                _ => "other",
            })
            .collect();
        assert_eq!(
            shape,
            ["pre-start", "program", "launching", "handoff"],
            "{posted:?}"
        );
        assert_eq!(harness.wait_for_marks(&["pre", "game"]), ["pre", "game"]);
    }

    #[test]
    fn a_failing_pre_start_script_keeps_the_game_from_starting_and_runs_the_close_script() {
        let _locale = locale("en");
        let mut harness = Harness::new();
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
        entry.pre_start_script =
            Some(harness.script("start.bat", "pre", &["echo port 80 is in use 1>&2"], 1));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));

        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );

        let WorkerResult::Error { title, message } = result else {
            panic!("{result:?}");
        };
        assert_eq!(title, "Could not launch Game");
        assert!(message.contains("exit code 1"), "{message}");
        assert!(message.ends_with("port 80 is in use"), "{message}");
        assert!(harness.context.active_watches.is_empty());
        assert_eq!(harness.wait_for_marks(&["pre", "close"]), ["pre", "close"]);
        assert_eq!(
            harness.marks_after(Duration::from_secs(1)),
            ["pre", "close"],
            "the game never started and the close script ran once"
        );
    }

    #[test]
    fn a_plugin_game_runs_the_pre_start_script_before_its_plugin_and_the_close_script_after() {
        let _locale = locale("en");
        let mut harness = Harness::new();
        let mut entry = game(&harness.absent("game.exe"));
        entry.plugin = Some(PluginAssignment {
            plugin_id: "org.example.absent".into(),
            revision: PluginRevision {
                version: semver::Version::parse("1.0.0").unwrap(),
                sha256: "a".repeat(64),
            },
            update_policy: PluginUpdatePolicy::Pinned,
            allow_prerelease: false,
            copy_zero_matches_to_game_folder: false,
            search_directory_destinations: false,
            assignment_id: "0123456789abcdef0123456789abcdef".into(),
            resolved_launcher: None,
            resolved_watch_target: None,
            grants: Vec::new(),
            ledger_reference: harness.absent("ledger.toml"),
        });
        entry.pre_start_script = Some(harness.script("start.bat", "pre", &[], 0));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));

        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );

        let WorkerResult::Error { title, .. } = result else {
            panic!("{result:?}");
        };
        assert_eq!(title, "Could not prepare Game");
        assert_eq!(harness.wait_for_marks(&["pre", "close"]), ["pre", "close"]);
    }

    #[test]
    fn a_failed_launch_without_a_pre_start_script_runs_no_close_script() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.absent("missing game.exe"));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));

        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );

        assert!(matches!(result, WorkerResult::Error { .. }), "{result:?}");
        assert!(harness.marks_after(Duration::from_millis(1500)).is_empty());
    }

    #[test]
    fn a_launch_cancelled_while_its_pre_start_script_runs_starts_no_game_and_runs_the_close_script()
    {
        let mut harness = Harness::new();
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
        entry.pre_start_script =
            Some(harness.script("start server.bat", "pre", &["ping -n 60 127.0.0.1 >nul"], 0));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));
        let cancel = LaunchCancel::new();
        let remote = cancel.clone();
        let marks = harness.marks_file();

        let operator = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while Instant::now() < deadline {
                if std::fs::read_to_string(&marks).is_ok_and(|text| text.contains("pre")) {
                    remote.cancel();
                    return true;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            false
        });
        let started = Instant::now();

        let result =
            harness
                .context
                .launch(entry.clone(), config(&entry), cancel, released_handoff());

        assert!(
            operator.join().unwrap(),
            "the script started and was cancelled"
        );
        assert!(
            matches!(result, WorkerResult::LaunchCancelled { game_id } if game_id == entry.id),
            "{result:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "not the script's minute"
        );
        assert!(
            harness.context.active_watches.is_empty(),
            "nothing is armed"
        );
        assert!(
            !harness
                .posted()
                .iter()
                .any(|result| matches!(result, WorkerResult::Error { .. })),
            "a cancel shows no error"
        );
        assert_eq!(harness.wait_for_marks(&["pre", "close"]), ["pre", "close"]);
        assert_eq!(
            harness.marks_after(Duration::from_secs(1)),
            ["pre", "close"],
            "the game never started and the close script ran once"
        );
    }

    #[test]
    fn a_launch_cancelled_before_it_starts_runs_nothing_at_all() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
        entry.pre_start_script = Some(harness.script("start.bat", "pre", &[], 0));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));
        let cancel = LaunchCancel::new();
        cancel.cancel();

        let result =
            harness
                .context
                .launch(entry.clone(), config(&entry), cancel, released_handoff());

        assert!(
            matches!(result, WorkerResult::LaunchCancelled { .. }),
            "{result:?}"
        );
        assert!(
            harness.marks_after(Duration::from_millis(1500)).is_empty(),
            "no pre-start script, no game, and no close script, since nothing ran"
        );
    }

    #[test]
    fn a_game_started_as_its_launch_is_cancelled_is_ended_and_nothing_is_armed() {
        let mut harness = Harness::new();
        let entry = game(&harness.lasting_game("game.bat", "game"));
        let settings = config(&entry).settings.console();
        let spawned = harness
            .context
            .runtime
            .launch(&entry, settings, &mut harness.context.processes)
            .unwrap();
        let root = crate::launcher_gui::process::ProcessWatch::open(spawned.pid)
            .expect("the started game is held");

        assert!(!harness
            .context
            .stop_if_cancelled(entry.id, &LaunchCancel::new()));
        assert!(root.exit_code().is_none());

        let cancel = LaunchCancel::new();
        cancel.cancel();
        assert!(harness.context.stop_if_cancelled(entry.id, &cancel));
        let deadline = Instant::now() + Duration::from_secs(10);
        while root.exit_code().is_none() {
            assert!(Instant::now() < deadline, "the cancelled game's tree ended");
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(harness.context.active_watches.is_empty());
    }

    #[test]
    fn the_console_watcher_hears_of_each_program_the_moment_it_starts() {
        let (mut harness, watched) = Harness::watched();
        let mut entry = game(&harness.lasting_game("game.bat", "game"));
        entry.pre_start_script = Some(harness.script("start.bat", "pre", &[], 0));

        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );

        let WorkerResult::Launched { root_pid, .. } = result else {
            panic!("{result:?}");
        };
        let followed = crate::launcher_gui::process::followed(&watched);
        assert_eq!(
            followed.len(),
            2,
            "the pre-start script, then the game: {followed:?}"
        );
        assert_eq!(followed[0].0, Some(entry.id));
        assert_ne!(followed[0].1, root_pid, "first the pre-start script");
        assert_eq!(
            followed[1],
            (Some(entry.id), root_pid),
            "then the game's root"
        );

        let stop = harness.script("stop.bat", "close", &[], 0);
        harness.context.start_close_script(&stop).unwrap();
        let followed = crate::launcher_gui::process::followed(&watched);
        assert!(
            matches!(followed.as_slice(), [(None, pid)] if *pid != 0),
            "{followed:?}"
        );
        let _ = harness.context.processes.stop_game(entry.id);
    }
}

mod operator_files {
    use std::fs;

    use super::super::super::fixtures::{config, game, released_handoff, Harness};
    use super::super::super::messages::WorkerResult;
    use crate::launcher_gui::model::{PluginAssignment, PluginRevision, PluginUpdatePolicy};
    use crate::launcher_gui::runtime::LaunchCancel;
    use crate::plugin::catalog::{CatalogRevision, ImportOutcome};
    use crate::plugin::manifest::{CopyType, Manifest, PluginFile};

    fn import_plugin(harness: &Harness) -> CatalogRevision {
        let package = harness.absent("package");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("payload.bin"), b"plugin").unwrap();
        let manifest = Manifest {
            manifest_version: 1,
            plugin_id: "org.example.operatorfiles".into(),
            plugin_version: semver::Version::parse("1.0.0").unwrap(),
            label: "Operator files".into(),
            description: "A plugin with one file in the game folder".into(),
            is_patcher: false,
            game_launcher: None,
            executable_watch: None,
            executable_watch_type: None,
            runtime: None,
            config_files: Vec::new(),
            plugin_files: vec![PluginFile {
                plugin_file_location: "payload.bin".into(),
                plugin_file_copy_type: CopyType::GameFolder,
                plugin_file_replace_path: None,
            }],
            install_scripts: Vec::new(),
        };
        fs::write(
            package.join("manifest.toml"),
            manifest.canonical_toml().unwrap(),
        )
        .unwrap();
        let backend = harness.context.plugin_backend.as_ref().unwrap();
        match backend
            .catalog
            .import_manifest(&package.join("manifest.toml"))
            .unwrap()
        {
            ImportOutcome::Imported(revision) | ImportOutcome::AlreadyPresent(revision) => revision,
        }
    }

    #[test]
    fn a_game_whose_plugin_file_the_operator_changed_starts_and_the_log_names_the_file() {
        let mut harness = Harness::new();
        let revision = import_plugin(&harness);
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
        entry.plugin = Some(PluginAssignment {
            plugin_id: revision.plugin_id.clone(),
            revision: PluginRevision {
                version: revision.plugin_version.clone(),
                sha256: revision.package_sha256.clone(),
            },
            update_policy: PluginUpdatePolicy::TrackLatest,
            allow_prerelease: false,
            copy_zero_matches_to_game_folder: false,
            search_directory_destinations: false,
            assignment_id: "0123456789abcdef0123456789abcdef".into(),
            resolved_launcher: None,
            resolved_watch_target: None,
            grants: Vec::new(),
            ledger_reference: harness.absent("ledger.toml"),
        });
        let payload = entry.selected_target.with_file_name("payload.bin");
        harness.context.log.set_enabled(true);
        let kept_lines = |harness: &Harness| -> Vec<String> {
            fs::read_to_string(harness.context.launcher_dir.join("GameToaster.log"))
                .unwrap()
                .lines()
                .filter(|line| line.contains("operation=plugin-file-kept"))
                .map(str::to_owned)
                .collect()
        };

        let launched = harness.context.launch(
            entry.clone(),
            config(&entry),
            LaunchCancel::new(),
            released_handoff(),
        );
        let WorkerResult::Launched { updated_config, .. } = launched else {
            panic!("{launched:?}");
        };
        assert_eq!(fs::read(&payload).unwrap(), b"plugin");
        harness.pump_until(|result| matches!(result, WorkerResult::GameExited(_)));
        assert!(kept_lines(&harness).is_empty(), "nothing was kept yet");

        fs::write(&payload, b"operator-build").unwrap();
        let stored = updated_config
            .expect("the first launch records the assignment")
            .games[0]
            .clone();
        let launched = harness.context.launch(
            stored.clone(),
            config(&stored),
            LaunchCancel::new(),
            released_handoff(),
        );
        assert!(
            matches!(launched, WorkerResult::Launched { .. }),
            "{launched:?}"
        );
        harness.pump_until(|result| matches!(result, WorkerResult::GameExited(_)));
        assert_eq!(fs::read(&payload).unwrap(), b"operator-build");
        assert_eq!(harness.marks(), ["game", "game"]);

        let kept = kept_lines(&harness);
        assert_eq!(kept.len(), 1, "{kept:?}");
        assert!(kept[0].contains(r#"game_id="1""#), "{}", kept[0]);
        assert!(kept[0].contains(r#"payload.bin""#), "{}", kept[0]);
    }
}
