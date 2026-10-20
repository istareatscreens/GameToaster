use super::*;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::super::channel::messages::{
    GameMode, GameState, LogLevel, LogMessage, Progress, ResultStatus, Stage,
};
use crate::support::process::fixtures::{ends_within, hold};
use crate::support::process::handle::{creation_time, ProcessHandle};
use crate::support::process::tree::direct_children;

pub(crate) const HIDDEN: ConsoleSettings = ConsoleSettings {
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

fn with_console(mut spec: OperationSpec, console: ConsoleSettings) -> OperationSpec {
    spec.console = console;
    spec
}

fn held_program(wrapper: u32, created: CreationTime, pid: u32) -> Option<ProcessHandle> {
    direct_children(wrapper, created)
        .into_iter()
        .find(|child| child.pid() == pid)
}

fn wait_for_file(path: &Path) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(text) = fs::read_to_string(path) {
            if !text.is_empty() {
                return text;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{} never appeared",
            path.display()
        );
        thread::sleep(Duration::from_millis(20));
    }
}

impl OperationHandle {
    fn cancel(&self) -> Result<(), String> {
        self.cancel.cancel()
    }
}

impl RuntimeHost {
    fn run_to_completion(spec: OperationSpec) -> Result<ChildResult, RuntimeEvent> {
        let handle = Self::start(spec).map_err(RuntimeEvent::WorkerFailure)?;
        loop {
            match handle.recv().map_err(RuntimeEvent::WorkerFailure)? {
                RuntimeEvent::Exited(result) => return Ok(result),
                RuntimeEvent::TimedOut => return Err(RuntimeEvent::TimedOut),
                RuntimeEvent::LauncherCancelled => return Err(RuntimeEvent::LauncherCancelled),
                RuntimeEvent::WorkerFailure(error) => {
                    return Err(RuntimeEvent::WorkerFailure(error))
                }
                RuntimeEvent::Started { .. }
                | RuntimeEvent::Connected { .. }
                | RuntimeEvent::Message(_)
                | RuntimeEvent::Invalid(_)
                | RuntimeEvent::Ignored { .. } => {}
            }
        }
    }

    fn prepare_then_launch(
        mut spec: OperationSpec,
    ) -> Result<(ChildResult, ChildResult), RuntimeEvent> {
        spec.operation = Operation::Prepare;
        let prepare = Self::run_to_completion(spec.clone())?;
        if !prepare.succeeded() {
            return Err(RuntimeEvent::Exited(prepare));
        }
        spec.operation = Operation::Launch;
        let launch = Self::run_to_completion(spec)?;
        Ok((prepare, launch))
    }
}

fn remove_stale_fake_plugins(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .starts_with("GameToaster-fake-plugin-v2-")
        {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > Duration::from_secs(3_600));
        if old {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

pub(crate) fn fake_plugin() -> PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let parent = std::env::temp_dir();
        remove_stale_fake_plugins(&parent);
        let root = parent.join(format!("GameToaster-fake-plugin-v2-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("fake_plugin.rs");
        let executable = root.join("fake_plugin.exe");
        fs::write(
            &source,
            r#"
use std::io::Write;
use std::time::Duration;

fn env(name: &str) -> Option<String> { std::env::var(name).ok().filter(|v| !v.is_empty()) }

fn open_pipe() -> Option<std::fs::File> {
    let name = env("GAMETOASTER_PIPE")?;
    for _ in 0..50 {
        if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&name) {
            return Some(file);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

fn write_lines(pipe: &mut Option<std::fs::File>, lines: &str) {
    if let Some(pipe) = pipe {
        for line in lines.split('|') {
            let _ = pipe.write_all(line.as_bytes());
            let _ = pipe.write_all(b"\n");
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--as-child") {
        let mut pipe = open_pipe();
        write_lines(&mut pipe, &env("FAKE_CHILD_LINES").unwrap_or_default());
        return;
    }
    if let Some(path) = env("FAKE_PID_FILE") {
        std::fs::write(path, std::process::id().to_string()).unwrap();
    }
    let request_path = args
        .windows(2)
        .find(|pair| pair[0] == "--gametoaster")
        .map(|pair| pair[1].clone())
        .expect("the request switch");
    let request = std::fs::read_to_string(&request_path).expect("the request file");
    if let Some(copy) = env("FAKE_REQUEST_COPY") {
        std::fs::write(copy, &request).unwrap();
    }
    let operation = ["prepare", "launch", "restore"]
        .into_iter()
        .find(|name| request.contains(&format!("\"operation\": \"{name}\"")))
        .unwrap_or("unknown");
    if operation == "launch" {
        if let Some(marker) = env("FAKE_LAUNCH_MARKER") {
            std::fs::write(marker, b"launched").unwrap();
        }
    }
    let mut pipe = open_pipe();
    if let Some(lines) = env("FAKE_LINES") {
        write_lines(&mut pipe, &lines);
    }
    if env("FAKE_CHILD_LINES").is_some() {
        let status = std::process::Command::new(&args[0]).arg("--as-child").status().unwrap();
        assert!(status.success());
    }
    if let Some(message) = env("FAKE_ERROR") {
        eprintln!("{message}");
    }
    let delay = env("FAKE_DELAY_MS").and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);
    std::thread::sleep(Duration::from_millis(delay));
    let code = if operation == "prepare" { env("FAKE_PREPARE_EXIT") } else { env("FAKE_EXIT") }
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    drop(pipe);
    std::process::exit(code);
}
"#,
        )
        .unwrap();
        let status = Command::new("rustc")
            .arg("--edition=2021")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap();
        assert!(status.success());
        executable
    })
    .clone()
}

fn fake_gui_plugin() -> PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let root = fake_plugin().parent().unwrap().to_path_buf();
        let source = root.join("fake_gui_plugin.rs");
        let executable = root.join("fake_gui_plugin.exe");
        fs::write(
            &source,
            "#![windows_subsystem = \"windows\"]\nfn main() {\n    eprintln!(\"the graphical plugin failed\");\n    std::process::exit(3);\n}\n",
        )
        .unwrap();
        let status = Command::new("rustc")
            .arg("--edition=2021")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap();
        assert!(status.success());
        executable
    })
    .clone()
}

pub(crate) fn spec(root: &Path, timeout: Duration) -> OperationSpec {
    OperationSpec {
        executable: fake_plugin(),
        working_directory: root.to_path_buf(),
        session_state_root: root.join("sessions"),
        operation: Operation::Prepare,
        request_context: RequestContext {
            plugin_id: "org.example.fake".into(),
            plugin_version: "1.0.0".into(),
            plugin_sha256: "a".repeat(64),
            numeric_game_id: 1,
            assignment_id: "b".repeat(32),
            game_folder: root.to_path_buf(),
            selected_target: root.join("game with space.exe"),
            resolved_launcher: None,
        },
        timeout,
        environment: Vec::new(),
        console: HIDDEN,
        request_repaint: None,
    }
}

pub(crate) fn with_env(mut spec: OperationSpec, pairs: &[(&str, &str)]) -> OperationSpec {
    spec.environment.extend(
        pairs
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into())),
    );
    spec
}

fn run(spec: OperationSpec) -> Vec<RuntimeEvent> {
    let handle = RuntimeHost::start(spec).unwrap();
    let mut events = Vec::new();
    loop {
        let event = handle.recv().unwrap();
        let terminal = matches!(
            event,
            RuntimeEvent::Exited(_)
                | RuntimeEvent::TimedOut
                | RuntimeEvent::LauncherCancelled
                | RuntimeEvent::WorkerFailure(_)
        );
        events.push(event);
        if terminal {
            break;
        }
    }
    handle.join().unwrap();
    events
}

fn messages(events: &[RuntimeEvent]) -> Vec<Message> {
    events
        .iter()
        .filter_map(|event| match event {
            RuntimeEvent::Message(message) => Some(message.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn messages_arrive_in_order_and_exit_zero_succeeds_with_the_result() {
    let root = tempfile::tempdir().unwrap();
    let repaints = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&repaints);
    let mut operation = with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[(
            "FAKE_LINES",
            r#"{"type":"progress","stage":"patching","percent":40,"text":"Patching"}|{"type":"game_state","credits":1,"mode":"ready"}|{"type":"result","status":"ok","text":"done"}"#,
        )],
    );
    operation.request_repaint = Some(Arc::new(move || {
        counter.fetch_add(1, Ordering::Relaxed);
    }));
    let events = run(operation);
    assert!(matches!(events[0], RuntimeEvent::Started { .. }));
    assert!(matches!(events[1], RuntimeEvent::Connected { client_pid } if client_pid != 0));
    assert_eq!(
        messages(&events),
        [
            Message::Progress(Progress {
                stage: Stage::Patching,
                percent: Some(40.0),
                text: "Patching".into(),
            }),
            Message::GameState(GameState {
                credits: Some(1),
                paid_play: None,
                free_play: false,
                mode: GameMode::Ready,
            }),
            Message::Result(ResultMessage {
                status: ResultStatus::Ok,
                text: "done".into(),
            }),
        ]
    );
    let RuntimeEvent::Exited(result) = events.last().unwrap() else {
        panic!("{events:?}");
    };
    assert!(result.succeeded());
    assert_eq!(result.code, 0);
    assert_eq!(result.result.as_ref().unwrap().text, "done");
    assert!(repaints.load(Ordering::Relaxed) >= 3);
}

#[test]
fn the_started_event_names_the_plugin_process_and_when_it_was_created() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid.txt");
    let handle = RuntimeHost::start(with_env(
        spec(root.path(), Duration::from_secs(30)),
        &[
            ("FAKE_DELAY_MS", "1500"),
            ("FAKE_PID_FILE", pid_file.to_str().unwrap()),
        ],
    ))
    .unwrap();
    let RuntimeEvent::Started {
        process_id,
        created,
    } = handle.recv().unwrap()
    else {
        panic!("the first event is the start");
    };
    assert_eq!((process_id, created), (handle.process_id, handle.created));
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !pid_file.is_file() {
        assert!(std::time::Instant::now() < deadline, "the plugin never ran");
        thread::sleep(Duration::from_millis(20));
    }
    let running = crate::support::process::snapshot::snapshot()
        .into_iter()
        .any(|entry| entry.pid == process_id);
    assert!(running);
    assert_eq!(
        fs::read_to_string(&pid_file).unwrap(),
        process_id.to_string()
    );
    let opened = unsafe {
        windows::Win32::System::Threading::OpenProcess(
            windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    }
    .unwrap();
    assert_eq!(creation_time(opened), Some(created));
    unsafe {
        let _ = CloseHandle(opened);
    }
    while !matches!(handle.recv().unwrap(), RuntimeEvent::Exited(_)) {}
    handle.join().unwrap();
}

#[test]
fn a_nonzero_exit_fails_with_the_result_text_or_else_the_output_tail() {
    let root = tempfile::tempdir().unwrap();
    let events = run(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[
            (
                "FAKE_LINES",
                r#"{"type":"result","status":"refused","text":"target is already patched"}"#,
            ),
            ("FAKE_PREPARE_EXIT", "20"),
            ("FAKE_ERROR", "a line on standard error"),
        ],
    ));
    let RuntimeEvent::Exited(result) = events.last().unwrap() else {
        panic!("{events:?}");
    };
    assert!(!result.succeeded());
    assert_eq!(result.code, 20);
    assert_eq!(
        result.result.as_ref().unwrap().status,
        ResultStatus::Refused
    );
    assert_eq!(
        result.reason().as_deref(),
        Some("target is already patched")
    );
    assert_eq!(result.output.as_deref(), Some("a line on standard error"));

    let events = run(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[
            ("FAKE_PREPARE_EXIT", "3"),
            ("FAKE_ERROR", "only standard error"),
        ],
    ));
    let RuntimeEvent::Exited(result) = events.last().unwrap() else {
        panic!("{events:?}");
    };
    assert_eq!((result.code, result.result.clone()), (3, None));
    assert_eq!(result.reason().as_deref(), Some("only standard error"));
}

#[test]
fn an_ok_result_cannot_turn_a_nonzero_exit_into_success() {
    let root = tempfile::tempdir().unwrap();
    let result = RuntimeHost::run_to_completion(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[
            ("FAKE_LINES", r#"{"type":"result","status":"ok"}"#),
            ("FAKE_PREPARE_EXIT", "7"),
        ],
    ))
    .unwrap();
    assert!(!result.succeeded());
    assert_eq!(result.result.unwrap().status, ResultStatus::Ok);
}

#[test]
fn unknown_oversized_and_garbage_lines_are_each_told_once_and_do_not_end_the_operation() {
    let root = tempfile::tempdir().unwrap();
    let long = "z".repeat(70_000);
    let lines = format!(
        r#"{{"type":"future","x":1}}|{long}|garbage|{{"type":"log","level":"warn","text":"still here"}}"#
    );
    let events = run(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[("FAKE_LINES", &lines)],
    ));
    assert_eq!(
        messages(&events),
        [Message::Log(LogMessage {
            level: LogLevel::Warn,
            text: "still here".into(),
        })]
    );

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeEvent::Ignored { .. }))
            .collect::<Vec<_>>(),
        [&RuntimeEvent::Ignored {
            message_type: "future".into()
        }]
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, RuntimeEvent::Invalid(_)))
            .collect::<Vec<_>>(),
        [&RuntimeEvent::Invalid(Invalid::Overlong)]
    );
    assert!(matches!(events.last(), Some(RuntimeEvent::Exited(result)) if result.succeeded()));
}

#[test]
fn the_plugin_and_a_process_it_starts_both_deliver() {
    let root = tempfile::tempdir().unwrap();
    let events = run(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[
            ("FAKE_LINES", r#"{"type":"progress","stage":"launching"}"#),
            (
                "FAKE_CHILD_LINES",
                r#"{"type":"game_state","credits":2,"paidPlay":true}"#,
            ),
        ],
    ));
    let connected = events
        .iter()
        .filter(|event| matches!(event, RuntimeEvent::Connected { .. }))
        .count();
    assert_eq!(connected, 2);
    assert!(messages(&events).contains(&Message::GameState(GameState {
        credits: Some(2),
        paid_play: Some(true),
        free_play: false,
        mode: GameMode::Unknown,
    })));
    assert!(matches!(events.last(), Some(RuntimeEvent::Exited(result)) if result.succeeded()));
}

#[test]
fn a_plugin_that_sends_nothing_still_succeeds() {
    let root = tempfile::tempdir().unwrap();
    let result =
        RuntimeHost::run_to_completion(spec(root.path(), Duration::from_secs(10))).unwrap();
    assert_eq!(
        result,
        ChildResult {
            code: 0,
            result: None,
            output: None,
        }
    );
}

#[test]
fn timeout_and_cancellation_are_launcher_results() {
    let root = tempfile::tempdir().unwrap();
    let hanging = with_env(
        spec(root.path(), Duration::from_millis(200)),
        &[("FAKE_DELAY_MS", "5000")],
    );
    assert_eq!(
        RuntimeHost::run_to_completion(hanging).unwrap_err(),
        RuntimeEvent::TimedOut
    );
    let handle = RuntimeHost::start(with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[("FAKE_DELAY_MS", "5000")],
    ))
    .unwrap();
    assert!(matches!(
        handle.recv().unwrap(),
        RuntimeEvent::Started { .. }
    ));
    handle.cancel().unwrap();
    let mut last = handle.recv().unwrap();
    while !matches!(last, RuntimeEvent::LauncherCancelled) {
        last = handle.recv().unwrap();
    }
}

#[test]
fn failed_prepare_never_launches() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("launch-marker.txt");
    let operation = with_env(
        spec(root.path(), Duration::from_secs(5)),
        &[
            ("FAKE_PREPARE_EXIT", "30"),
            ("FAKE_LAUNCH_MARKER", marker.to_str().unwrap()),
        ],
    );
    let result = RuntimeHost::prepare_then_launch(operation).unwrap_err();
    assert!(matches!(
        result,
        RuntimeEvent::Exited(ChildResult { code: 30, .. })
    ));
    assert!(!marker.exists());
    let launched = with_env(
        spec(root.path(), Duration::from_secs(5)),
        &[("FAKE_LAUNCH_MARKER", marker.to_str().unwrap())],
    );
    let (prepare, launch) = RuntimeHost::prepare_then_launch(launched).unwrap();
    assert!(prepare.succeeded() && launch.succeeded());
    assert!(marker.exists());
}

#[test]
fn the_plugin_receives_the_contract_request_and_the_channel_in_its_environment() {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("received request.json");
    let mut operation = with_env(
        spec(root.path(), Duration::from_secs(10)),
        &[("FAKE_REQUEST_COPY", copy.to_str().unwrap())],
    );
    operation.request_context.resolved_launcher = Some(root.path().join("plugin launcher.exe"));
    let events = run(operation);
    assert!(matches!(events.last(), Some(RuntimeEvent::Exited(result)) if result.succeeded()));
    let request: super::super::request::Request =
        serde_json::from_slice(&fs::read(&copy).unwrap()).unwrap();
    assert_eq!(request.protocol, 2);
    assert_eq!(request.operation, Operation::Prepare);
    assert!(request.pipe.starts_with(r"\\.\pipe\GameToaster.v2."));
    assert!(request.game.executable.ends_with("game with space.exe"));
    assert!(request
        .game
        .launcher
        .unwrap()
        .ends_with("plugin launcher.exe"));
    assert_eq!(request.plugin.id, "org.example.fake");
    assert!(!root
        .path()
        .join("sessions")
        .join(&request.operation_id)
        .exists());
}

#[test]
fn shown_a_failure_is_explained_by_the_result_message_since_nothing_is_captured() {
    let root = tempfile::tempdir().unwrap();
    let failed = |lines: &str| {
        let mut pairs = vec![
            ("FAKE_PREPARE_EXIT", "20"),
            ("FAKE_ERROR", "a line on standard error"),
        ];
        if !lines.is_empty() {
            pairs.push(("FAKE_LINES", lines));
        }
        RuntimeHost::run_to_completion(with_console(
            with_env(spec(root.path(), Duration::from_secs(30)), &pairs),
            SHOWN,
        ))
        .unwrap()
    };
    let unexplained = failed("");
    assert_eq!(
        (unexplained.code, unexplained.output.as_deref()),
        (20, None)
    );
    assert_eq!(unexplained.reason(), None);
    let explained = failed(r#"{"type":"result","status":"refused","text":"already patched"}"#);
    assert_eq!(explained.code, 20);
    assert_eq!(explained.reason().as_deref(), Some("already patched"));
}

#[test]
fn paused_a_success_runs_through_the_wrapper_without_pausing() {
    let root = tempfile::tempdir().unwrap();
    let events = run(with_console(
        with_env(
            spec(root.path(), Duration::from_secs(30)),
            &[("FAKE_LINES", r#"{"type":"progress","stage":"patching"}"#)],
        ),
        PAUSE,
    ));
    assert!(messages(&events).contains(&Message::Progress(Progress {
        stage: Stage::Patching,
        percent: None,
        text: String::new(),
    })));
    assert!(matches!(
        events.last(),
        Some(RuntimeEvent::Exited(result)) if result.code == 0 && result.output.is_none()
    ));
}

#[test]
fn a_graphical_plugin_is_captured_even_with_command_windows_shown() {
    let root = tempfile::tempdir().unwrap();
    for console in [SHOWN, PAUSE] {
        let mut operation = with_console(spec(root.path(), Duration::from_secs(30)), console);
        operation.executable = fake_gui_plugin();
        let events = run(operation);
        let Some(RuntimeEvent::Exited(result)) = events.last() else {
            panic!("{events:?}");
        };
        assert_eq!(result.code, 3, "{console:?}");
        assert!(
            result
                .output
                .as_deref()
                .is_some_and(|output| output.contains("the graphical plugin failed")),
            "{console:?}: {:?}",
            result.output
        );
    }
}

#[test]
fn paused_a_cancel_ends_both_the_wrapper_and_the_program() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid.txt");
    let handle = RuntimeHost::start(with_console(
        with_env(
            spec(root.path(), Duration::from_secs(60)),
            &[
                ("FAKE_DELAY_MS", "60000"),
                ("FAKE_PID_FILE", pid_file.to_str().unwrap()),
            ],
        ),
        PAUSE,
    ))
    .unwrap();
    let program: u32 = wait_for_file(&pid_file).parse().unwrap();
    let program = held_program(handle.process_id, handle.created, program)
        .expect("the program runs under the wrapper");
    let wrapper = hold(handle.process_id);
    assert!(!program.has_exited() && !wrapper.has_exited());
    handle.cancel().unwrap();
    let mut last = handle.recv().unwrap();
    while !matches!(last, RuntimeEvent::LauncherCancelled) {
        last = handle.recv().unwrap();
    }
    handle.join().unwrap();
    assert!(program.has_exited(), "the program ended");
    assert!(wrapper.has_exited(), "the wrapper ended");
}

#[test]
fn paused_a_program_still_running_at_the_deadline_times_out_through_a_junction() {
    let root = tempfile::tempdir().unwrap();
    let junction = root.path().join("plugin");
    let made = Command::new(crate::support::process::interpreter::comspec())
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&junction)
        .arg(fake_plugin().parent().unwrap())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(made.success());
    let pid_file = root.path().join("pid.txt");
    let timeout = Duration::from_secs(3);
    let mut operation = with_console(
        with_env(
            spec(root.path(), timeout),
            &[
                ("FAKE_DELAY_MS", "60000"),
                ("FAKE_PID_FILE", pid_file.to_str().unwrap()),
            ],
        ),
        PAUSE,
    );
    operation.executable = junction.join(fake_plugin().file_name().unwrap());
    let handle = RuntimeHost::start(operation).unwrap();
    let program: u32 = wait_for_file(&pid_file).parse().unwrap();
    let program = held_program(handle.process_id, handle.created, program)
        .expect("the program runs under the wrapper");
    let wrapper = hold(handle.process_id);

    let deadline = std::time::Instant::now() + timeout + Duration::from_secs(15);
    let ended = loop {
        match handle.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(RuntimeEvent::TimedOut) => break true,
            Ok(RuntimeEvent::Exited(result)) => panic!("exited with {}", result.code),
            Ok(_) | Err(_) if std::time::Instant::now() < deadline => {}
            Ok(_) | Err(_) => break false,
        }
    };
    if !ended {
        handle.cancel().unwrap();
    }
    handle.join().unwrap();
    assert!(
        ended,
        "a hung program under the pause wrapper never timed out"
    );
    assert!(program.has_exited(), "the program ended");
    assert!(wrapper.has_exited(), "the wrapper ended");
}

#[test]
fn paused_a_program_started_after_the_deadline_is_timed_out() {
    let root = tempfile::tempdir().unwrap();
    let handle = RuntimeHost::start(with_console(
        with_env(
            spec(root.path(), Duration::from_millis(1)),
            &[("FAKE_DELAY_MS", "60000")],
        ),
        PAUSE,
    ))
    .unwrap();
    let (wrapper_id, created) = (handle.process_id, handle.created);

    let wrapper = hold(wrapper_id);

    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let timed_out = loop {
        match handle.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(RuntimeEvent::TimedOut) => break true,
            Ok(RuntimeEvent::Exited(result)) => panic!("exited with {}", result.code),
            Ok(_) | Err(_) if std::time::Instant::now() < deadline => {}
            Ok(_) | Err(_) => break false,
        }
    };
    if !timed_out {
        handle.cancel().unwrap();
    }
    handle.join().unwrap();
    assert!(
        timed_out,
        "a program started after the deadline was never timed out"
    );
    assert!(wrapper.has_exited(), "the wrapper ended");

    let program = fake_plugin();
    assert!(
        !direct_children(wrapper_id, created)
            .iter()
            .any(|child| child
                .image_path()
                .is_some_and(|image| crate::support::paths::same_file(&image, &program))),
        "the program ended"
    );
}

#[test]
fn paused_the_timeout_never_counts_the_wait_for_a_key() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid.txt");

    let timeout = Duration::from_secs(10);
    let operation = with_console(
        with_env(
            spec(root.path(), timeout),
            &[
                ("FAKE_PREPARE_EXIT", "20"),
                ("FAKE_PID_FILE", pid_file.to_str().unwrap()),
            ],
        ),
        PAUSE,
    );
    let started = std::time::Instant::now();
    let handle = RuntimeHost::start(operation).unwrap();

    let program: u32 = wait_for_file(&pid_file).parse().unwrap();

    if let Some(program) = held_program(handle.process_id, handle.created, program) {
        assert!(ends_within(&program, timeout), "the program never ended");
    }
    assert!(
        started.elapsed() < timeout,
        "the program ended before the timeout"
    );
    while started.elapsed() < timeout + Duration::from_secs(2) {
        match handle.receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(RuntimeEvent::TimedOut) => {
                panic!("the wait for a key counted as the plugin's time")
            }

            Ok(RuntimeEvent::Exited(result)) => {
                assert_eq!(result.code, 20);
                handle.join().unwrap();
                return;
            }
            Ok(_) | Err(_) => {}
        }
    }
    handle.cancel().unwrap();
    let mut last = handle.recv().unwrap();
    while !matches!(
        last,
        RuntimeEvent::LauncherCancelled | RuntimeEvent::Exited(_)
    ) {
        assert!(!matches!(last, RuntimeEvent::TimedOut));
        last = handle.recv().unwrap();
    }
    handle.join().unwrap();
}
