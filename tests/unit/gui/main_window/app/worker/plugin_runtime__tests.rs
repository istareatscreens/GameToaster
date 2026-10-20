use super::*;
use crate::gui::common::harness::locale;
use crate::plugin::runtime::channel::messages::{ResultMessage, ResultStatus};
use crate::plugin::runtime::ChildResult;

const OPERATIONS: [RuntimeOperation; 3] = [
    RuntimeOperation::Prepare,
    RuntimeOperation::Launch,
    RuntimeOperation::Restore,
];

#[test]
fn an_operation_ends_in_one_language() {
    for code in ["ja", "zh-Hans", "zh-Hant"] {
        let _locale = locale(code);
        for operation in OPERATIONS {
            let name = operation_name(operation);
            let texts = [
                t!("plugins.error.runtime_cancelled", operation => name).to_string(),
                t!("plugins.error.operation_timeout", operation => name).to_string(),
                t!("plugins.error.operation_exit", operation => name, code => 3).to_string(),
                t!("plugins.error.operation_refused", operation => name, code => 20).to_string(),
            ];
            for text in texts {
                assert!(
                    !text.chars().any(|c| c.is_ascii_alphabetic()),
                    "{code} {operation:?}: {text}"
                );
            }
        }
    }
}

#[test]
fn an_operation_ends_in_english_exactly_as_before() {
    let _locale = locale("en");
    let launch = operation_name(RuntimeOperation::Launch);
    assert_eq!(
        t!("plugins.error.runtime_cancelled", operation => launch),
        "plugin launch was cancelled"
    );
    let prepare = operation_name(RuntimeOperation::Prepare);
    assert_eq!(
        t!("plugins.error.operation_timeout", operation => prepare),
        "plugin prepare exceeded its timeout"
    );
    let restore = operation_name(RuntimeOperation::Restore);
    assert_eq!(
        t!("plugins.error.operation_exit", operation => restore, code => 3),
        "plugin restore failed (exit 3)"
    );
}

#[test]
fn a_failure_is_worded_by_the_result_message_or_the_output() {
    let _locale = locale("en");
    let refused = ChildResult {
        code: 20,
        result: Some(ResultMessage {
            status: ResultStatus::Refused,
            text: "target is already patched".into(),
        }),
        output: Some("ignored when a result was sent".into()),
    };
    assert_eq!(
        failure_text("prepare", &refused),
        "plugin prepare was refused (exit 20): target is already patched"
    );
    let failed = ChildResult {
        code: 30,
        result: None,
        output: Some("a line on standard error".into()),
    };
    assert_eq!(
        failure_text("prepare", &failed),
        "plugin prepare failed (exit 30): a line on standard error"
    );
    let silent = ChildResult {
        code: 1,
        result: None,
        output: None,
    };
    assert_eq!(
        failure_text("restore", &silent),
        "plugin restore failed (exit 1)"
    );
}

#[test]
fn a_plugin_operation_reports_its_program_before_it_ends() {
    use crate::plugin::runtime::host::tests::{spec, with_env};

    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid.txt");
    let operation = with_env(
        spec(root.path(), Duration::from_secs(30)),
        &[("FAKE_PID_FILE", pid_file.to_str().unwrap())],
    );
    let (results, received) = mpsc::channel();
    let log = LauncherLog::open(root.path()).unwrap();
    run_runtime_operation(
        GameId(1),
        operation,
        &crate::gui::common::game_state::GameStateHub::default(),
        &results,
        &egui::Context::default(),
        &log,
    )
    .unwrap();
    let reported: Vec<WorkerResult> = received.try_iter().collect();
    let Some(WorkerResult::ProgramStarted {
        process_id,
        created,
    }) = reported.first()
    else {
        panic!("the program is reported first: {reported:?}");
    };
    assert_eq!(
        std::fs::read_to_string(&pid_file).unwrap(),
        process_id.to_string()
    );
    assert_ne!(*created, 0);
    assert!(matches!(
        reported.last(),
        Some(WorkerResult::PluginOperationEnded { .. })
    ));
}

#[test]
fn plugin_operations_start_only_where_their_program_is_reported() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gui");
    let mut starts = Vec::new();
    let mut folders = vec![root.clone()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).unwrap();
                for _ in source.matches("RuntimeHost::start(") {
                    starts.push(
                        path.strip_prefix(&root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
        }
    }
    starts.sort();
    assert_eq!(
        starts,
        [
            "main_window/app/worker/launch.rs",
            "main_window/app/worker/plugin_runtime.rs"
        ]
    );
    let launch = std::fs::read_to_string(root.join("main_window/app/worker/launch.rs")).unwrap();
    assert_eq!(launch.matches("root_created").count(), 3);
}

mod builders {
    use super::*;
    use crate::launcher_gui::model::{
        CloseMode, PluginAssignment, PluginRevision, PluginUpdatePolicy,
    };
    use crate::plugin::catalog::CatalogRevision;
    use crate::plugin::deployment::{AssignmentLedger, AssignmentState};
    use crate::plugin::lifecycle::ResolvedRuntime;
    use crate::plugin::paths::DeploymentPlan;
    use std::path::PathBuf;

    const SETTINGS: [ConsoleSettings; 3] = [
        ConsoleSettings {
            show_command_windows: false,
            pause_on_failure: false,
        },
        ConsoleSettings {
            show_command_windows: true,
            pause_on_failure: false,
        },
        ConsoleSettings {
            show_command_windows: false,
            pause_on_failure: true,
        },
    ];

    fn folder() -> PathBuf {
        std::env::temp_dir().join("GameToaster builder fixture")
    }

    fn game() -> GameEntry {
        GameEntry {
            id: GameId(7),
            label: "Game".into(),
            controller_image: None,
            selected_target: folder().join("game.exe"),
            plugin: Some(PluginAssignment {
                plugin_id: "org.example.plugin".into(),
                revision: PluginRevision {
                    version: semver::Version::parse("1.2.0").unwrap(),
                    sha256: "f".repeat(64),
                },
                update_policy: PluginUpdatePolicy::Pinned,
                allow_prerelease: false,
                copy_zero_matches_to_game_folder: true,
                search_directory_destinations: false,
                assignment_id: "0123456789abcdef0123456789abcdef".into(),
                resolved_launcher: None,
                resolved_watch_target: None,
                grants: Vec::new(),
                ledger_reference: folder().join("ledger.toml"),
            }),
            hotkey: None,
            watch_exe: None,
            close_mode: CloseMode::KillExe,
            close_script: None,
            close_hotkey: None,
            config_targets: Vec::new(),
        }
    }

    fn ledger() -> AssignmentLedger {
        AssignmentLedger {
            ledger_version: 1,
            assignment_id: "0123456789abcdef0123456789abcdef".into(),
            plugin_id: "org.example.plugin".into(),
            plugin_version: "1.2.0".into(),
            revision_sha256: "f".repeat(64),
            game_id: 7,
            game_folder: folder(),
            selected_target: folder().join("game.exe"),
            resolved_launcher: None,
            resolved_watch_target: None,
            state: AssignmentState::Active,
            files: Vec::new(),
            retired_files: Vec::new(),
            transactions: Vec::new(),
            install_script_receipts: Vec::new(),
        }
    }

    fn revision() -> CatalogRevision {
        CatalogRevision {
            plugin_id: "org.example.plugin".into(),
            plugin_version: semver::Version::parse("1.2.0").unwrap(),
            package_md5: "e".repeat(32),
            package_sha256: "f".repeat(64),
            imported_at: 0,
            root: folder(),
        }
    }

    fn runtime() -> ResolvedRuntime {
        ResolvedRuntime {
            executable: folder().join("plugin").join("runtime.exe"),
            operations: vec![
                ManifestOperation::Prepare,
                ManifestOperation::Launch,
                ManifestOperation::Restore,
            ],
            is_launcher: false,
        }
    }

    #[test]
    fn every_builder_carries_the_console_settings_it_is_given() {
        let context = egui::Context::default();
        let session = folder().join("sessions");
        for console in SETTINGS {
            let from_ledger = runtime_spec_from_ledger(
                &game(),
                &ledger(),
                &runtime(),
                RuntimeOperation::Restore,
                &session,
                Duration::from_secs(300),
                console,
                &context,
            )
            .unwrap();
            let supported = runtime_spec_for_supported_operation(
                &game(),
                &ledger(),
                Some(&runtime()),
                RuntimeOperation::Prepare,
                &session,
                console,
                &context,
            )
            .unwrap()
            .unwrap();
            let planned = runtime_spec_for_planned_operation(
                &game(),
                &revision(),
                "0123456789abcdef0123456789abcdef",
                &DeploymentPlan {
                    plan_version: 1,
                    plugin_id: "org.example.plugin".into(),
                    revision_sha256: "f".repeat(64),
                    game_id: 7,
                    game_folder: folder(),
                    selected_target: folder().join("game.exe"),
                    resolved_launcher: None,
                    resolved_watch_target: None,
                    files: Vec::new(),
                },
                Some(&runtime()),
                RuntimeOperation::Prepare,
                &session,
                console,
                &context,
            )
            .unwrap()
            .unwrap();
            let launch = runtime_spec(
                &game(),
                &ReconcileResult {
                    revision: revision(),
                    ledger: ledger(),
                    ledger_reference: folder().join("ledger.toml"),
                    config_targets: Vec::new(),
                    runtime: Some(runtime()),
                    changed: false,
                },
                &runtime(),
                RuntimeOperation::Launch,
                &session,
                Duration::from_secs(60),
                console,
                &context,
            )
            .unwrap();
            for spec in [from_ledger, supported, planned, launch] {
                assert_eq!(spec.console, console, "{:?}", spec.operation);
            }
        }
    }
}

#[test]
fn a_plugins_log_line_and_an_invalid_line_reach_the_log_when_diagnostics_are_on() {
    use crate::plugin::runtime::channel::messages::{LogLevel, LogMessage};
    use crate::plugin::runtime::channel::Invalid;

    let root = tempfile::tempdir().unwrap();
    let log = LauncherLog::open(root.path()).unwrap();
    let written = || std::fs::read_to_string(root.path().join("GameToaster.log")).unwrap();
    let line = RuntimeEvent::Message(Message::Log(LogMessage {
        level: LogLevel::Warn,
        text: "the receipt could not be written".into(),
    }));

    log_runtime_event(&log, GameId(7), "operation-1", &line);
    assert_eq!(written(), "", "diagnostics logging is off by default");

    log.set_enabled(true);
    log_runtime_event(&log, GameId(7), "operation-1", &line);
    log_runtime_event(
        &log,
        GameId(7),
        "operation-1",
        &RuntimeEvent::Invalid(Invalid::Field("stage")),
    );
    let lines: Vec<String> = written().lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].contains("operation=plugin-log"), "{}", lines[0]);
    assert!(lines[0].contains(r#"level="warn""#), "{}", lines[0]);
    assert!(
        lines[0].contains(r#"text="the receipt could not be written""#),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].contains("operation=plugin-channel-invalid"),
        "{}",
        lines[1]
    );
    assert!(lines[1].contains("stage"), "{}", lines[1]);
}
