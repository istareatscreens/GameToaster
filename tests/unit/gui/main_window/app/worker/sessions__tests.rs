use super::*;

fn watch(policy: CompletionPolicy, job_released: bool, target_exited: bool) -> ActiveWatch {
    ActiveWatch {
        policy,
        job_released,
        target_exited,
        terminal_sent: false,
        _target: None,
        _plugin_activity: None,
        _generic_activity: None,
        plugin_cancel: None,
        plugin_root: None,
        runtime_thread: None,
        label: "Game".into(),
        close_script: None,
        watched: None,
        program_watch: None,
    }
}

#[test]
fn completion_is_authoritative_per_policy_and_never_fabricated() {
    assert!(!watch(CompletionPolicy::Job, false, true).is_terminal());
    assert!(watch(CompletionPolicy::Job, true, false).is_terminal());

    assert!(!watch(CompletionPolicy::EscapedProcess, true, false).is_terminal());
    assert!(watch(CompletionPolicy::EscapedProcess, false, true).is_terminal());

    assert!(!watch(CompletionPolicy::FailedWatch, true, true).is_terminal());

    assert!(!watch(CompletionPolicy::PluginRuntime, true, false).is_terminal());
    assert!(watch(CompletionPolicy::PluginRuntime, false, true).is_terminal());
}

mod scripts {
    use std::time::Duration;

    use super::super::super::fixtures::{config, game, released_handoff, Harness};
    use super::super::super::messages::WorkerResult;
    use super::super::{CompletionPolicy, WatchedProgram};
    use super::watch;
    use crate::launcher_gui::model::GameId;

    #[test]
    fn a_game_that_exits_by_itself_runs_its_close_script_once_after_its_end() {
        let mut harness = Harness::new();
        let mut entry = game(&harness.script("game.bat", "game", &[], 0));
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

        let posted = harness.pump_until(|result| matches!(result, WorkerResult::GameExited(_)));
        assert!(posted
            .iter()
            .any(|result| matches!(result, WorkerResult::GameExited(GameId(1)))));
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
    fn shutdown_runs_the_close_script_only_of_a_game_confirmed_gone() {
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

        let running = std::env::current_exe().unwrap();
        let mut outliving = watch(CompletionPolicy::EscapedProcess, false, false);
        outliving.close_script = Some(harness.script("stop other.bat", "other", &[], 0));
        outliving.watched = Some(WatchedProgram {
            name: running.file_name().unwrap().to_string_lossy().into_owned(),
            path: Some(running),
        });
        harness.context.active_watches.insert(GameId(2), outliving);

        harness.context.shutdown();

        assert!(harness
            .posted()
            .iter()
            .any(|result| matches!(result, WorkerResult::ShutdownComplete(Ok(())))));
        assert_eq!(
            harness.wait_for_marks(&["game", "close"]),
            ["game", "close"]
        );
        assert_eq!(
            harness.marks_after(Duration::from_secs(1)),
            ["game", "close"],
            "the outliving game kept what its pre-start script started"
        );
    }
}
