use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use super::*;
use crate::gui::common::harness::locale;
use crate::launcher_gui::process::ProcessWatch;
use crate::support::process::handle::ProcessHandle;

const HIDDEN: ConsoleSettings = ConsoleSettings {
    show_command_windows: false,
    pause_on_failure: false,
};

fn script(folder: &Path, name: &str, body: &[&str]) -> PathBuf {
    let path = folder.join(name);
    let mut text = String::from("@echo off\r\n");
    for line in body {
        text.push_str(line);
        text.push_str("\r\n");
    }
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_script_that_succeeds_lets_the_launch_go_on_and_is_reported_first() {
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let start = script(
        scripts.path(),
        "start server.bat",
        &["echo started", "exit /b 0"],
    );
    let runtime = RuntimeService::new(state.path().to_path_buf());
    let reported = Cell::new(None);

    let outcome = runtime.run_pre_start_script(
        GameId(4),
        &start,
        HIDDEN,
        Duration::from_secs(30),
        &LaunchCancel::new(),
        |pid, _created| reported.set(Some(pid)),
    );

    assert_eq!(outcome, Ok(()));
    assert!(reported.get().is_some_and(|pid| pid != 0));

    let captured =
        fs::read_to_string(state.path().join("script_output/4-pre-start.out.txt")).unwrap();
    assert_eq!(captured.trim(), "started");
}

#[test]
fn a_failing_script_names_its_exit_code_and_its_output_errors_first() {
    let _locale = locale("en");
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let start = script(
        scripts.path(),
        "start.bat",
        &[
            "echo checking the ports",
            "echo port 80 is in use 1>&2",
            "exit /b 3",
        ],
    );
    let runtime = RuntimeService::new(state.path().to_path_buf());

    let failure = runtime
        .run_pre_start_script(
            GameId(1),
            &start,
            HIDDEN,
            Duration::from_secs(30),
            &LaunchCancel::new(),
            |_, _| {},
        )
        .unwrap_err();

    assert_eq!(
        failure,
        PreStartFailure::Failed {
            code: 3,
            output: Some("port 80 is in use\nchecking the ports".into()),
        }
    );
    let message = failure.message(&start);
    assert!(message.contains(&start.display().to_string()), "{message}");
    assert!(message.contains("exit code 3"), "{message}");
    assert!(
        message.ends_with("port 80 is in use\nchecking the ports"),
        "{message}"
    );
}

#[test]
fn a_missing_script_is_not_started() {
    let _locale = locale("en");
    let state = tempfile::tempdir().unwrap();
    let missing = state.path().join("absent.bat");
    let runtime = RuntimeService::new(state.path().to_path_buf());
    let mut reported = false;

    let failure = runtime
        .run_pre_start_script(
            GameId(1),
            &missing,
            HIDDEN,
            Duration::from_secs(30),
            &LaunchCancel::new(),
            |_, _| reported = true,
        )
        .unwrap_err();

    assert!(
        matches!(failure, PreStartFailure::NotStarted(_)),
        "{failure:?}"
    );
    assert!(!reported, "nothing started, so nothing is reported");
    let message = failure.message(&missing);
    assert!(message.contains("could not be started"), "{message}");

    let relative = runtime.run_pre_start_script(
        GameId(1),
        Path::new("start.bat"),
        HIDDEN,
        Duration::from_secs(30),
        &LaunchCancel::new(),
        |_, _| {},
    );
    assert!(matches!(relative, Err(PreStartFailure::NotStarted(_))));
}

#[test]
fn a_script_that_never_ends_is_ended_at_its_deadline() {
    let _locale = locale("en");
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let endless = script(scripts.path(), "endless.bat", &[":again", "goto again"]);
    let runtime = RuntimeService::new(state.path().to_path_buf());
    let reported = Cell::new(None);
    let started = Instant::now();

    let failure = runtime
        .run_pre_start_script(
            GameId(1),
            &endless,
            HIDDEN,
            Duration::from_millis(1500),
            &LaunchCancel::new(),
            |pid, _| reported.set(Some(pid)),
        )
        .unwrap_err();

    assert_eq!(failure, PreStartFailure::TimedOut);
    assert!(started.elapsed() >= Duration::from_millis(1500));
    let pid = reported.get().expect("the script was reported");
    assert!(
        ProcessWatch::open(pid).is_none_or(|process| process.exit_code().is_some()),
        "the timed-out script no longer runs"
    );
    assert!(
        failure.message(&endless).contains("10 minutes"),
        "{}",
        failure.message(&endless)
    );
}

#[test]
fn the_script_runs_in_its_own_folder() {
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let start = script(scripts.path(), "start.bat", &["cd> where.txt"]);
    let runtime = RuntimeService::new(state.path().to_path_buf());

    runtime
        .run_pre_start_script(
            GameId(1),
            &start,
            HIDDEN,
            Duration::from_secs(30),
            &LaunchCancel::new(),
            |_, _| {},
        )
        .unwrap();

    let written = fs::read_to_string(scripts.path().join("where.txt")).unwrap();
    assert!(
        crate::support::paths::same_file(Path::new(written.trim()), scripts.path()),
        "{written:?}"
    );
}

fn child_named(parent: u32, image: &str, within: Duration) -> Option<ProcessHandle> {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        let found = crate::support::process::tree::direct_children(parent, 0)
            .into_iter()
            .find(|child| {
                child
                    .image_path()
                    .and_then(|path| path.file_name().map(|name| name.to_os_string()))
                    .is_some_and(|name| name.eq_ignore_ascii_case(image))
            });
        if found.is_some() {
            return found;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

fn run_with_child(
    script: &Path,
    deadline: Duration,
    cancel_after_child: bool,
) -> (Result<(), PreStartFailure>, u32, Option<ProcessHandle>) {
    let state = tempfile::tempdir().unwrap();
    let runtime = RuntimeService::new(state.path().to_path_buf());
    let cancel = LaunchCancel::new();
    let (pid_sender, pid_receiver) = std::sync::mpsc::channel::<u32>();
    let remote = cancel.clone();
    let helper = std::thread::spawn(move || {
        let pid = pid_receiver.recv().ok()?;
        let child = child_named(pid, "PING.EXE", Duration::from_secs(10));
        if cancel_after_child {
            remote.cancel();
        }
        child
    });
    let reported = Cell::new(0);
    let outcome =
        runtime.run_pre_start_script(GameId(1), script, HIDDEN, deadline, &cancel, |pid, _| {
            reported.set(pid);
            let _ = pid_sender.send(pid);
        });
    let child = helper.join().unwrap();
    (outcome, reported.get(), child)
}

#[test]
fn a_cancelled_launch_ends_the_script_and_every_program_it_started() {
    let scripts = tempfile::tempdir().unwrap();
    let start = script(
        scripts.path(),
        "start.bat",
        &["ping -n 60 127.0.0.1 >nul", "exit /b 0"],
    );
    let started = Instant::now();

    let (outcome, pid, child) = run_with_child(&start, Duration::from_secs(60), true);

    assert_eq!(outcome, Err(PreStartFailure::Cancelled));
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the cancel ended the wait at once, not the script's minute"
    );
    let child = child.expect("the script's child was seen before the cancel");
    assert!(child.has_exited(), "the script's child was ended");
    assert!(
        ProcessWatch::open(pid).is_none_or(|process| process.exit_code().is_some()),
        "the script was ended"
    );
}

#[test]
fn a_script_past_its_deadline_is_ended_with_what_it_started() {
    let scripts = tempfile::tempdir().unwrap();
    let start = script(
        scripts.path(),
        "slow.bat",
        &["ping -n 60 127.0.0.1 >nul", "exit /b 0"],
    );

    let (outcome, _, child) = run_with_child(&start, Duration::from_millis(2500), false);

    assert_eq!(outcome, Err(PreStartFailure::TimedOut));
    let child = child.expect("the script's child was seen before the deadline");
    assert!(
        child.has_exited(),
        "a timed-out script leaves no program it started running"
    );
}

#[test]
fn a_launch_cancelled_before_its_script_never_starts_it() {
    let _locale = locale("en");
    let state = tempfile::tempdir().unwrap();
    let scripts = tempfile::tempdir().unwrap();
    let start = script(scripts.path(), "start.bat", &["echo ran> ran.txt"]);
    let runtime = RuntimeService::new(state.path().to_path_buf());
    let cancel = LaunchCancel::new();
    cancel.cancel();
    let mut reported = false;

    let outcome = runtime.run_pre_start_script(
        GameId(1),
        &start,
        HIDDEN,
        Duration::from_secs(30),
        &cancel,
        |_, _| reported = true,
    );

    assert_eq!(outcome, Err(PreStartFailure::Cancelled));
    assert!(!reported, "nothing was started");
    assert!(!scripts.path().join("ran.txt").exists());
    assert_eq!(
        PreStartFailure::Cancelled.message(&start),
        "The launch was cancelled."
    );
}
