use std::time::{Duration, Instant};

use super::*;
use crate::plugin::runtime::host::tests::fake_plugin;

const HIDDEN: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: false,
};
const SHOWN: ConsoleSettings = ConsoleSettings {
    show_command_windows: true,
    pause_on_failure: false,
};
const PAUSE: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: true,
};

fn start(folder: &Path, console: ConsoleSettings, environment: &[(&str, &str)]) -> PluginProcess {
    let environment: Vec<(OsString, OsString)> = environment
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect();
    let request = folder.join("request.json");
    std::fs::write(&request, "{\"operation\": \"prepare\"}").unwrap();
    let arguments = [OsString::from("--gametoaster"), request.into_os_string()];
    PluginProcess::start(
        StartSpec {
            executable: &fake_plugin(),
            arguments: &arguments,
            working_directory: folder,
            environment: &environment,
            console,
        },
        || {
            let stdout = File::create(folder.join("stdout.txt")).map_err(|e| e.to_string())?;
            let stderr = File::create(folder.join("stderr.txt")).map_err(|e| e.to_string())?;
            Ok((stdout, stderr))
        },
    )
    .unwrap()
}

fn wait_for(process: &PluginProcess) {
    let wait = unsafe { WaitForSingleObject(process.handle(), 30_000) };
    assert_eq!(wait, windows::Win32::Foundation::WAIT_OBJECT_0);
}

#[test]
fn hidden_the_plugin_has_no_window_and_its_output_is_captured() {
    let folder = tempfile::tempdir().unwrap();
    let mut process = start(
        folder.path(),
        HIDDEN,
        &[
            ("FAKE_PREPARE_EXIT", "20"),
            ("FAKE_ERROR", "a line on standard error"),
        ],
    );
    assert!(process.captures_output());
    assert_ne!(process.created(), 0);
    wait_for(&process);
    assert_eq!(process.exit_code().unwrap(), 20);
    assert_eq!(
        std::fs::read_to_string(folder.path().join("stderr.txt"))
            .unwrap()
            .trim(),
        "a line on standard error"
    );
}

#[test]
fn shown_the_plugin_runs_in_a_console_of_its_own_and_nothing_is_captured() {
    let folder = tempfile::tempdir().unwrap();
    let mut process = start(folder.path(), SHOWN, &[("FAKE_PREPARE_EXIT", "7")]);
    assert!(!process.captures_output());
    assert!(
        !folder.path().join("stdout.txt").exists(),
        "no capture file is opened"
    );
    wait_for(&process);
    assert_eq!(process.exit_code().unwrap(), 7);
}

#[test]
fn kill_ends_the_plugin_and_reports_it_no_longer_runs() {
    let folder = tempfile::tempdir().unwrap();
    for console in [HIDDEN, SHOWN] {
        let mut process = start(folder.path(), console, &[("FAKE_DELAY_MS", "30000")]);
        assert_eq!(
            process.program_state(),
            ProgramState::Running,
            "{console:?}"
        );
        let started = Instant::now();
        process.kill();
        wait_for(&process);
        assert!(started.elapsed() < Duration::from_secs(10), "{console:?}");
        assert_eq!(process.program_state(), ProgramState::Ended, "{console:?}");
    }
}

#[test]
fn paused_the_program_is_held_from_the_first_look_until_it_has_ended() {
    let folder = tempfile::tempdir().unwrap();
    let mut process = start(folder.path(), PAUSE, &[("FAKE_DELAY_MS", "30000")]);

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut state = process.program_state();
    while state == ProgramState::Unseen {
        assert!(
            Instant::now() < deadline,
            "the wrapper never started the program"
        );
        std::thread::sleep(Duration::from_millis(20));
        state = process.program_state();
    }
    assert_eq!(state, ProgramState::Running);
    process.kill();
    wait_for(&process);

    assert_eq!(process.program_state(), ProgramState::Ended);
}
