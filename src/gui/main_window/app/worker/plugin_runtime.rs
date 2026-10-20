use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use eframe::egui;
use rust_i18n::t;

use super::messages::{WorkerCommand, WorkerResult};
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::{GameEntry, GameId};
use crate::plugin::lifecycle::{runtime_executable_candidates, ReconcileResult};
use crate::plugin::manifest::Operation as ManifestOperation;
use crate::plugin::runtime::{
    Message, Operation as RuntimeOperation, OperationHandle, OperationSpec, RequestContext,
    RuntimeEvent, RuntimeHost,
};
use crate::support::paths::paths_equal;
use crate::support::process::console_policy::ConsoleSettings;

pub(super) fn runtime_for_ledger(
    manifest: &crate::plugin::manifest::Manifest,
    ledger: &crate::plugin::deployment::AssignmentLedger,
) -> Result<Option<crate::plugin::lifecycle::ResolvedRuntime>, String> {
    let Some(runtime) = &manifest.runtime else {
        return Ok(None);
    };
    let matches = runtime_executable_candidates(
        runtime,
        ledger
            .files
            .iter()
            .map(|file| (file.source_relative.as_str(), file.destination.as_path())),
    )?;
    let executable = match matches.as_slice() {
        [path] if path.is_file() => path.clone(),
        [path] => {
            return Err(t!(
                "plugins.error.runtime_missing",
                path => path.display().to_string()
            )
            .to_string())
        }
        _ => return Err(t!("plugins.error.runtime_ambiguous").to_string()),
    };
    Ok(Some(crate::plugin::lifecycle::ResolvedRuntime {
        is_launcher: ledger
            .resolved_launcher
            .as_ref()
            .is_some_and(|launcher| paths_equal(launcher, &executable)),
        executable,
        operations: runtime.operations.clone(),
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_spec_from_ledger(
    game: &GameEntry,
    ledger: &crate::plugin::deployment::AssignmentLedger,
    runtime: &crate::plugin::lifecycle::ResolvedRuntime,
    operation: RuntimeOperation,
    session_root: &Path,
    timeout: Duration,
    console: ConsoleSettings,
    ctx: &egui::Context,
) -> Result<OperationSpec, String> {
    let assignment = game
        .plugin
        .as_ref()
        .ok_or_else(|| t!("plugins.error.runtime_unassigned").to_string())?;
    let working_directory = runtime
        .executable
        .parent()
        .ok_or_else(|| t!("plugins.error.runtime_no_parent").to_string())?
        .to_path_buf();
    let repaint = ctx.clone();
    Ok(OperationSpec {
        executable: runtime.executable.clone(),
        working_directory,
        session_state_root: session_root.to_path_buf(),
        operation,
        request_context: RequestContext {
            plugin_id: assignment.plugin_id.clone(),
            plugin_version: assignment.revision.version.to_string(),
            plugin_sha256: assignment.revision.sha256.clone(),
            numeric_game_id: game.id.0,
            assignment_id: assignment.assignment_id.clone(),
            game_folder: ledger.game_folder.clone(),
            selected_target: game.selected_target.clone(),
            resolved_launcher: ledger.resolved_launcher.clone(),
        },
        timeout,
        environment: Vec::new(),
        console,
        request_repaint: Some(Arc::new(move || repaint.request_repaint())),
    })
}

pub(super) fn runtime_for_plan(
    manifest: &crate::plugin::manifest::Manifest,
    plan: &crate::plugin::paths::DeploymentPlan,
) -> Result<Option<crate::plugin::lifecycle::ResolvedRuntime>, String> {
    let Some(runtime) = &manifest.runtime else {
        return Ok(None);
    };
    let matches = runtime_executable_candidates(
        runtime,
        plan.files
            .iter()
            .map(|file| (file.source_relative.as_str(), file.destination.as_path())),
    )?;
    let executable = match matches.as_slice() {
        [path] => path.clone(),
        _ => return Err(t!("plugins.error.runtime_ambiguous_in_plan").to_string()),
    };
    Ok(Some(crate::plugin::lifecycle::ResolvedRuntime {
        is_launcher: plan
            .resolved_launcher
            .as_ref()
            .is_some_and(|launcher| paths_equal(launcher, &executable)),
        executable,
        operations: runtime.operations.clone(),
    }))
}

pub(super) fn runtime_spec_for_supported_operation(
    game: &GameEntry,
    ledger: &crate::plugin::deployment::AssignmentLedger,
    runtime: Option<&crate::plugin::lifecycle::ResolvedRuntime>,
    operation: RuntimeOperation,
    session_root: &Path,
    console: ConsoleSettings,
    ctx: &egui::Context,
) -> Result<Option<OperationSpec>, String> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };
    if !runtime_supports(runtime, operation) {
        return Ok(None);
    }
    runtime_spec_from_ledger(
        game,
        ledger,
        runtime,
        operation,
        session_root,
        Duration::from_secs(300),
        console,
        ctx,
    )
    .map(Some)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_spec_for_planned_operation(
    game: &GameEntry,
    revision: &crate::plugin::catalog::CatalogRevision,
    assignment_id: &str,
    plan: &crate::plugin::paths::DeploymentPlan,
    runtime: Option<&crate::plugin::lifecycle::ResolvedRuntime>,
    operation: RuntimeOperation,
    session_root: &Path,
    console: ConsoleSettings,
    ctx: &egui::Context,
) -> Result<Option<OperationSpec>, String> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };
    if !runtime_supports(runtime, operation) {
        return Ok(None);
    }
    let working_directory = runtime
        .executable
        .parent()
        .ok_or_else(|| t!("plugins.error.planned_runtime_no_parent").to_string())?
        .to_path_buf();
    let repaint = ctx.clone();
    Ok(Some(OperationSpec {
        executable: runtime.executable.clone(),
        working_directory,
        session_state_root: session_root.to_path_buf(),
        operation,
        request_context: RequestContext {
            plugin_id: revision.plugin_id.clone(),
            plugin_version: revision.plugin_version.to_string(),
            plugin_sha256: revision.package_sha256.clone(),
            numeric_game_id: game.id.0,
            assignment_id: assignment_id.to_string(),
            game_folder: plan.game_folder.clone(),
            selected_target: game.selected_target.clone(),
            resolved_launcher: plan.resolved_launcher.clone(),
        },
        timeout: Duration::from_secs(300),
        environment: Vec::new(),
        console,
        request_repaint: Some(Arc::new(move || repaint.request_repaint())),
    }))
}

pub(super) fn runtime_supports(
    runtime: &crate::plugin::lifecycle::ResolvedRuntime,
    operation: RuntimeOperation,
) -> bool {
    let operation = match operation {
        RuntimeOperation::Prepare => ManifestOperation::Prepare,
        RuntimeOperation::Launch => ManifestOperation::Launch,
        RuntimeOperation::Restore => ManifestOperation::Restore,
    };
    runtime.operations.contains(&operation)
}

pub(super) fn run_optional_runtime(
    game_id: GameId,
    spec: Option<OperationSpec>,
    game_state: &crate::gui::common::game_state::GameStateHub,
    results: &mpsc::Sender<WorkerResult>,
    ctx: &egui::Context,
    log: &LauncherLog,
) -> Result<(), String> {
    match spec {
        Some(spec) => run_runtime_operation(game_id, spec, game_state, results, ctx, log),
        None => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn runtime_spec(
    game: &GameEntry,
    result: &ReconcileResult,
    runtime: &crate::plugin::lifecycle::ResolvedRuntime,
    operation: RuntimeOperation,
    session_root: &Path,
    timeout: Duration,
    console: ConsoleSettings,
    ctx: &egui::Context,
) -> Result<OperationSpec, String> {
    let assignment = game
        .plugin
        .as_ref()
        .ok_or_else(|| t!("plugins.error.runtime_unassigned").to_string())?;
    let working_directory = runtime
        .executable
        .parent()
        .ok_or_else(|| {
            t!(
                "plugins.error.runtime_no_parent_path",
                path => runtime.executable.display().to_string()
            )
            .to_string()
        })?
        .to_path_buf();
    let repaint = ctx.clone();
    Ok(OperationSpec {
        executable: runtime.executable.clone(),
        working_directory,
        session_state_root: session_root.to_path_buf(),
        operation,
        request_context: RequestContext {
            plugin_id: assignment.plugin_id.clone(),
            plugin_version: result.revision.plugin_version.to_string(),
            plugin_sha256: result.revision.package_sha256.clone(),
            numeric_game_id: game.id.0,
            assignment_id: assignment.assignment_id.clone(),
            game_folder: result.ledger.game_folder.clone(),
            selected_target: game.selected_target.clone(),
            resolved_launcher: result.ledger.resolved_launcher.clone(),
        },
        timeout,
        environment: Vec::new(),
        console,
        request_repaint: Some(Arc::new(move || repaint.request_repaint())),
    })
}

pub(super) fn failure_text(operation: &str, child: &crate::plugin::runtime::ChildResult) -> String {
    let refused = child
        .result
        .as_ref()
        .is_some_and(|result| result.status == crate::plugin::runtime::ResultStatus::Refused);
    let mut message = if refused {
        t!("plugins.error.operation_refused", operation => operation, code => child.code)
    } else {
        t!("plugins.error.operation_exit", operation => operation, code => child.code)
    }
    .to_string();
    if let Some(reason) = child.reason() {
        message.push_str(": ");
        message.push_str(&reason);
    }
    message
}

fn operation_name(operation: RuntimeOperation) -> String {
    match operation {
        RuntimeOperation::Prepare => t!("plugins.operation.prepare"),
        RuntimeOperation::Launch => t!("plugins.operation.launch"),
        RuntimeOperation::Restore => t!("plugins.operation.restore"),
    }
    .to_string()
}

pub(super) fn run_runtime_operation(
    game_id: GameId,
    spec: OperationSpec,
    game_state: &crate::gui::common::game_state::GameStateHub,
    results: &mpsc::Sender<WorkerResult>,
    ctx: &egui::Context,
    log: &LauncherLog,
) -> Result<(), String> {
    let operation = operation_name(spec.operation);
    let handle = RuntimeHost::start(spec)?;
    let outcome = loop {
        let event = match handle.recv() {
            Ok(event) => event,
            Err(error) => break Err(error),
        };
        log_runtime_event(log, game_id, &handle.operation_id, &event);
        match event {
            RuntimeEvent::Started {
                process_id,
                created,
            } => {
                let _ = results.send(WorkerResult::ProgramStarted {
                    process_id,
                    created,
                });
                ctx.request_repaint();
            }
            RuntimeEvent::Connected { .. }
            | RuntimeEvent::Invalid(_)
            | RuntimeEvent::Ignored { .. } => {}
            RuntimeEvent::Message(message) => {
                if let Message::GameState(state) = &message {
                    game_state.apply(state);
                }
                let _ = results.send(WorkerResult::PluginMessage { game_id, message });
                ctx.request_repaint();
            }
            RuntimeEvent::Exited(child) => {
                break if child.succeeded() {
                    Ok(())
                } else {
                    Err(failure_text(&operation, &child))
                };
            }
            RuntimeEvent::TimedOut => {
                break Err(
                    t!("plugins.error.operation_timeout", operation => operation).to_string(),
                );
            }
            RuntimeEvent::LauncherCancelled => {
                break Err(
                    t!("plugins.error.runtime_cancelled", operation => operation).to_string(),
                );
            }
            RuntimeEvent::WorkerFailure(error) => break Err(error),
        }
    };

    game_state.clear();
    let _ = results.send(WorkerResult::PluginOperationEnded { game_id });
    ctx.request_repaint();
    outcome
}

pub(super) fn log_runtime_event(
    log: &LauncherLog,
    game_id: GameId,
    operation_id: &str,
    event: &RuntimeEvent,
) {
    let (name, detail): (&str, Vec<(&str, String)>) = match event {
        RuntimeEvent::Started { process_id, .. } => (
            "plugin-runtime-started",
            vec![("pid", process_id.to_string())],
        ),
        RuntimeEvent::Connected { client_pid } => (
            "plugin-channel-connected",
            vec![("client_pid", client_pid.to_string())],
        ),
        RuntimeEvent::Message(Message::Progress(progress)) => (
            "plugin-progress",
            vec![("stage", progress.stage.as_str().to_string())],
        ),
        RuntimeEvent::Message(Message::Log(entry)) => (
            "plugin-log",
            vec![
                ("level", entry.level.as_str().to_string()),
                ("text", entry.text.clone()),
            ],
        ),
        RuntimeEvent::Invalid(reason) => (
            "plugin-channel-invalid",
            vec![("reason", format!("{reason:?}"))],
        ),
        RuntimeEvent::Ignored { message_type } => (
            "plugin-channel-ignored",
            vec![("type", message_type.clone())],
        ),
        RuntimeEvent::Message(_)
        | RuntimeEvent::Exited(_)
        | RuntimeEvent::TimedOut
        | RuntimeEvent::LauncherCancelled
        | RuntimeEvent::WorkerFailure(_) => return,
    };
    let mut fields = vec![
        ("game_id", game_id.to_string()),
        ("operation_id", operation_id.to_string()),
    ];
    fields.extend(detail);
    log.event(name, &fields);
}

pub(super) fn forward_runtime_events(
    handle: OperationHandle,
    game_id: GameId,
    sender: SyncSender<WorkerCommand>,
) -> Result<JoinHandle<()>, String> {
    thread::Builder::new()
        .name(format!("gametoaster-plugin-session-{}", game_id.0))
        .spawn(move || {
            let operation_id = handle.operation_id.clone();
            loop {
                let event = match handle.recv() {
                    Ok(event) => event,
                    Err(error) => RuntimeEvent::WorkerFailure(error),
                };
                let terminal = matches!(
                    event,
                    RuntimeEvent::Exited(_)
                        | RuntimeEvent::TimedOut
                        | RuntimeEvent::LauncherCancelled
                        | RuntimeEvent::WorkerFailure(_)
                );
                if sender
                    .send(WorkerCommand::PluginRuntimeEvent {
                        game_id,
                        operation_id: operation_id.clone(),
                        event,
                    })
                    .is_err()
                {
                    break;
                }
                if terminal {
                    break;
                }
            }
            let _ = handle.join();
        })
        .map_err(|error| t!("plugins.error.forwarder_start", error => error).to_string())
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/plugin_runtime__tests.rs"]
mod tests;
