use super::*;

#[test]
fn only_a_kill_that_ended_a_direct_launch_observed_the_exit() {
    assert!(exit_observed(SessionClosed::Removed));

    assert!(!exit_observed(SessionClosed::PluginCancelled));

    assert!(!exit_observed(SessionClosed::None));
}

mod scripts {
    use std::time::Duration;

    use super::super::super::fixtures::{config, game, released_handoff, Harness};
    use super::super::super::messages::WorkerResult;
    use crate::launcher_gui::model::GameId;

    fn absent_name() -> String {
        format!("gametoaster-absent-{}.exe", std::process::id())
    }

    #[test]
    fn closing_a_running_direct_launch_ends_it_and_then_runs_its_close_script_once() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.lasting_game("game.bat", "game"));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));
        let result = harness.context.launch(
            entry.clone(),
            config(&entry),
            crate::launcher_gui::runtime::LaunchCancel::new(),
            released_handoff(),
        );
        assert!(
            matches!(result, WorkerResult::Launched { .. }),
            "{result:?}"
        );
        assert_eq!(harness.wait_for_marks(&["game"]), ["game"]);

        let result = harness.context.close_game(entry);

        assert!(
            matches!(
                result,
                WorkerResult::Closed {
                    game_id: GameId(1),
                    exit_observed: true
                }
            ),
            "{result:?}"
        );
        assert!(harness.context.active_watches.is_empty());
        assert_eq!(
            harness.wait_for_marks(&["game", "close"]),
            ["game", "close"]
        );
        assert_eq!(
            harness.marks_after(Duration::from_secs(1)),
            ["game", "close"],
            "the close script ran once"
        );
    }

    #[test]
    fn closing_a_plugin_game_whose_plugin_restarts_it_succeeds() {
        use super::super::super::sessions::{ActiveWatch, CompletionPolicy, WatchedProgram};
        use crate::plugin::runtime::OperationCancel;
        use crate::support::process::handle::creation_time_of;

        let harness = Harness::new();
        let folder = tempfile::tempdir().unwrap();
        let system = std::env::var_os("SystemRoot").expect("SystemRoot is set");
        let game_exe = folder
            .path()
            .join(format!("gametoaster-restarted-{}.exe", std::process::id()));
        std::fs::copy(
            std::path::Path::new(&system)
                .join("System32")
                .join("PING.EXE"),
            &game_exe,
        )
        .unwrap();
        let plugin_script = folder.path().join("plugin.bat");
        std::fs::write(
            &plugin_script,
            format!(
                "@echo off\r\n:again\r\n\"{}\" -n 60 127.0.0.1 >nul\r\ngoto again\r\n",
                game_exe.display()
            ),
        )
        .unwrap();

        let mut plugin = std::process::Command::new("cmd.exe")
            .arg("/c")
            .arg(&plugin_script)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let plugin_created = creation_time_of(plugin.id()).unwrap();

        struct EndOnDrop(u32, crate::support::process::handle::CreationTime);
        impl Drop for EndOnDrop {
            fn drop(&mut self) {
                crate::support::process::tree::end_process_tree(self.0, self.1, 2_000);
            }
        }
        let _guard = EndOnDrop(plugin.id(), plugin_created);
        let mut entry = game(&game_exe);
        entry.watch_exe = Some(game_exe.display().to_string());
        let cancel = OperationCancel::standalone();
        let mut context = harness.context;
        context.active_watches.insert(
            entry.id,
            ActiveWatch {
                policy: CompletionPolicy::PluginRuntime,
                job_released: false,
                target_exited: false,
                terminal_sent: false,
                _target: None,
                _plugin_activity: None,
                _generic_activity: None,
                plugin_cancel: Some(cancel.clone()),
                plugin_root: Some((plugin.id(), plugin_created)),
                runtime_thread: None,
                label: entry.label.clone(),
                close_script: None,
                watched: Some(WatchedProgram {
                    name: entry.watch_target().unwrap(),
                    path: entry.watch_target_path(),
                }),
                program_watch: None,
            },
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while !crate::launcher_gui::process::is_process_path_running(&game_exe) {
            assert!(
                std::time::Instant::now() < deadline,
                "the stand-in plugin never started the game"
            );
            std::thread::sleep(Duration::from_millis(50));
        }

        let result = context.close_game(entry);

        assert!(
            matches!(
                result,
                WorkerResult::Closed {
                    game_id: GameId(1),
                    exit_observed: false
                }
            ),
            "{result:?}"
        );
        assert!(cancel.was_cancelled(), "the plugin's runtime was cancelled");
        assert!(
            plugin.try_wait().unwrap().is_some(),
            "the stand-in plugin was ended"
        );
        std::thread::sleep(Duration::from_millis(500));
        assert!(
            !crate::launcher_gui::process::is_process_path_running(&game_exe),
            "the game was not started again"
        );
    }

    #[test]
    fn closing_a_game_that_is_not_running_runs_nothing() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.absent(&absent_name()));
        entry.watch_exe = Some(absent_name());
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));

        let result = harness.context.close_game(entry);

        assert!(
            matches!(
                result,
                WorkerResult::Closed {
                    exit_observed: false,
                    ..
                }
            ),
            "{result:?}"
        );
        assert!(harness.marks_after(Duration::from_millis(1500)).is_empty());
    }

    #[test]
    fn the_close_hotkey_runs_no_close_script_for_a_game_that_is_not_running() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.absent(&absent_name()));
        entry.close_script = Some(harness.script("stop.bat", "close", &[], 0));

        let result = harness.context.close_active_games(vec![entry]);

        assert!(
            matches!(result, WorkerResult::ActiveGamesClosed),
            "{result:?}"
        );
        assert!(harness.marks_after(Duration::from_millis(1500)).is_empty());
    }
}
