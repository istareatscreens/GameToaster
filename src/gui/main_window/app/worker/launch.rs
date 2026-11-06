use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::{LaunchStage, WorkerCommand, WorkerResult};
use super::plugin_runtime::{forward_runtime_events, run_runtime_operation, runtime_spec};
use super::sessions::{ActiveWatch, CompletionPolicy, WatchedProgram};
use crate::launcher_gui::config;
use crate::launcher_gui::model::{
    GameEntry, GameId, LauncherConfig, PluginRevision, PluginUpdatePolicy,
};
use crate::launcher_gui::runtime::{HandoffGate, LaunchCancel, PreStartFailure, PRE_START_TIMEOUT};
use crate::plugin::deployment::ModificationDecision;
use crate::plugin::lifecycle::{ReconcileRequest, ReconcileResult, RevisionPolicy};
use crate::plugin::manifest::Operation as ManifestOperation;
use crate::plugin::paths::ResolutionChoices;
use crate::plugin::runtime::{Operation as RuntimeOperation, RuntimeHost};
use crate::support::process::console_policy::ConsoleSettings;

enum Start {
    Done(Box<WorkerResult>),

    Cancelled { started: bool },
}

impl WorkerContext {
    pub(super) fn launch(
        &mut self,
        game: GameEntry,
        config: LauncherConfig,
        cancel: LaunchCancel,
        handoff: HandoffGate,
    ) -> WorkerResult {
        let console = config.settings.console();
        let game_id = game.id;
        let label = game.label.clone();
        let close_script = game.close_script.clone();
        let pre_start = game.pre_start_script.clone();
        let pre_start_ran = Cell::new(false);
        let outcome = match &pre_start {
            None => self.start(game, config, console, &cancel, &handoff),
            Some(script) => {
                match self.run_pre_start_script(&game, script, console, &cancel, &pre_start_ran) {
                    Ok(()) => self.start(game, config, console, &cancel, &handoff),
                    Err(PreStartFailure::Cancelled) => Start::Cancelled { started: false },
                    Err(failure) => Start::Done(Box::new(WorkerResult::Error {
                        title: t!("errors.launch_failed", game => label).to_string(),
                        message: failure.message(script),
                    })),
                }
            }
        };
        let outcome = match outcome {
            Start::Done(result)
                if matches!(*result, WorkerResult::Error { .. }) && cancel.is_cancelled() =>
            {
                Start::Cancelled { started: false }
            }
            outcome => outcome,
        };
        let (mut result, run_close_script) = match outcome {
            Start::Done(result) => {
                let stopped = matches!(*result, WorkerResult::Error { .. });
                (*result, stopped && pre_start.is_some())
            }
            Start::Cancelled { started } => (
                WorkerResult::LaunchCancelled { game_id },
                started || pre_start_ran.get(),
            ),
        };
        if run_close_script {
            if let Some(close_script) = close_script {
                if let Err(error) = self.start_close_script(&close_script) {
                    if let WorkerResult::Error { message, .. } = &mut result {
                        message.push_str("\n\n");
                        message.push_str(&t!("errors.close_script_failed", game => label));
                        message.push_str(": ");
                        message.push_str(&error);
                    } else {
                        self.log.event(
                            "close-script-error",
                            &[("game_id", game_id.to_string()), ("error", error)],
                        );
                    }
                }
            }
        }
        result
    }

    fn run_pre_start_script(
        &self,
        game: &GameEntry,
        script: &Path,
        console: ConsoleSettings,
        cancel: &LaunchCancel,
        ran: &Cell<bool>,
    ) -> Result<(), PreStartFailure> {
        self.report(WorkerResult::LaunchStage(LaunchStage::PreStartScript));
        let outcome = self.runtime.run_pre_start_script(
            game.id,
            script,
            console,
            PRE_START_TIMEOUT,
            cancel,
            |process_id, created| {
                ran.set(true);
                self.watch_program(Some(game.id), process_id, created);
                self.report(WorkerResult::ProgramStarted {
                    game_id: Some(game.id),
                    process_id,
                    created,
                })
            },
        );
        if outcome.is_ok() {
            self.report(WorkerResult::LaunchStage(LaunchStage::Launching));
        }
        outcome
    }

    fn start(
        &mut self,
        mut game: GameEntry,
        mut config: LauncherConfig,
        console: ConsoleSettings,
        cancel: &LaunchCancel,
        handoff: &HandoffGate,
    ) -> Start {
        if cancel.is_cancelled() {
            return Start::Cancelled { started: false };
        }
        let mut prepared_plugin = None;
        let mut generic_activity = None;
        let mut updated_config = None;
        let mut runtime_launch = None;
        let preparation = (|| -> Result<(), String> {
            let lifecycle = &self
                .plugin_backend
                .as_ref()
                .map_err(Clone::clone)?
                .lifecycle;
            if game.plugin.is_some() {
                let prepared = lifecycle.reconcile_for_launch(reconcile_request(&game)?)?;
                let record_changed = apply_reconciliation(&mut game, &prepared.reconciliation)?;
                let stored = config
                    .games
                    .iter_mut()
                    .find(|stored| stored.id == game.id)
                    .ok_or_else(|| {
                        t!("errors.game_disappeared_before_launch", id => game.id).to_string()
                    })?;
                *stored = game.clone();
                if record_changed {
                    config::save_atomic(&self.config_path, &config)?;
                }
                if record_changed || prepared.reconciliation.changed {
                    updated_config = Some(config.clone());
                }

                if let Some(plugin_runtime) = prepared.reconciliation.runtime.as_ref() {
                    let prepare_spec = runtime_spec(
                        &game,
                        &prepared.reconciliation,
                        plugin_runtime,
                        RuntimeOperation::Prepare,
                        &self.session_root,
                        Duration::from_secs(300),
                        console,
                        &self.ctx,
                    )?;
                    run_runtime_operation(
                        game.id,
                        prepare_spec,
                        Some(cancel),
                        &self.game_state,
                        &self.results,
                        &self.ctx,
                        &self.log,
                    )?;
                    if plugin_runtime.is_launcher
                        && plugin_runtime
                            .operations
                            .contains(&ManifestOperation::Launch)
                    {
                        runtime_launch = Some(runtime_spec(
                            &game,
                            &prepared.reconciliation,
                            plugin_runtime,
                            RuntimeOperation::Launch,
                            &self.session_root,
                            Duration::from_secs(7 * 24 * 60 * 60),
                            console,
                            &self.ctx,
                        )?);
                    }
                }
                prepared_plugin = Some(prepared);
            } else {
                generic_activity = Some(lifecycle.activity().reserve_game_activity()?);
            }
            Ok(())
        })();
        if let Err(message) = preparation {
            self.game_state.clear();
            return Start::Done(Box::new(WorkerResult::Error {
                title: t!("errors.prepare_failed", game => game.label).to_string(),
                message,
            }));
        }

        if cancel.is_cancelled() {
            self.game_state.clear();
            return Start::Cancelled { started: false };
        }

        self.report(WorkerResult::HandoffRequired { game_id: game.id });
        if let Err(message) = handoff.request(&|| cancel.is_cancelled()) {
            self.game_state.clear();
            if cancel.is_cancelled() {
                return Start::Cancelled { started: false };
            }
            return Start::Done(Box::new(WorkerResult::Error {
                title: t!("controller.dialog.release_failed_title").to_string(),
                message: t!("controller.dialog.release_failed_body", detail => message).to_string(),
            }));
        }
        Start::Done(Box::new(if let Some(spec) = runtime_launch {
            match RuntimeHost::start(spec) {
                Ok(handle) => {
                    let root_pid = handle.process_id;
                    let root_created = handle.created;
                    self.watch_program(Some(game.id), root_pid, root_created);
                    let cancel = handle.cancel_token();
                    match forward_runtime_events(handle, game.id, self.internal_sender.clone()) {
                        Ok(runtime_thread) => {
                            self.active_watches.insert(
                                game.id,
                                ActiveWatch {
                                    policy: CompletionPolicy::PluginRuntime,
                                    job_released: false,
                                    target_exited: false,
                                    terminal_sent: false,
                                    _target: None,
                                    _plugin_activity: prepared_plugin.take(),
                                    _generic_activity: None,
                                    plugin_cancel: Some(cancel),
                                    plugin_root: Some((root_pid, root_created)),
                                    runtime_thread: Some(runtime_thread),
                                    label: game.label.clone(),
                                    close_script: game.close_script.clone(),
                                    watched: game.watch_target().map(|name| WatchedProgram {
                                        name,
                                        path: game.watch_target_path(),
                                    }),
                                    program_watch: None,
                                },
                            );
                            WorkerResult::Launched {
                                game_id: game.id,
                                root_pid,
                                root_created,
                                updated_config,
                            }
                        }
                        Err(message) => WorkerResult::Error {
                            title: t!("errors.monitor_failed", game => game.label).to_string(),
                            message,
                        },
                    }
                }
                Err(message) => WorkerResult::Error {
                    title: t!("errors.launch_failed", game => game.label).to_string(),
                    message,
                },
            }
        } else {
            let watch_target = game.watch_target();
            let watch_path = game.watch_target_path();
            let escaped_watch = watch_target.as_ref().map(|name| {
                let baseline = watch_path
                    .as_deref()
                    .map(crate::launcher_gui::process::pids_by_path)
                    .unwrap_or_else(|| crate::launcher_gui::process::pids_by_name(name))
                    .into_iter()
                    .collect();
                (name.clone(), watch_path.clone(), baseline)
            });
            match self.runtime.launch(&game, console, &mut self.processes) {
                Ok(spawned) => {
                    self.watch_program(Some(game.id), spawned.pid, spawned.created);
                    if self.stop_if_cancelled(game.id, cancel) {
                        return Start::Cancelled { started: true };
                    }
                    let target = escaped_watch.and_then(|(name, path, baseline)| {
                        let event_sender = self.internal_sender.clone();
                        let wake = self.ctx.clone();
                        crate::launcher_gui::process::EscapedProcessWatch::start(
                            game.id,
                            name,
                            path,
                            baseline,
                            Arc::new(move |event| {
                                let _ = event_sender.send(WorkerCommand::WatchEvent(event));

                                wake.request_repaint();
                            }),
                        )
                        .inspect_err(|error| {
                            self.log.event(
                                "process-watch-attach-error",
                                &[("game_id", game.id.to_string()), ("error", error.clone())],
                            );
                        })
                        .ok()
                    });

                    let policy = match (watch_target.is_some(), target.is_some()) {
                        (false, _) => CompletionPolicy::Job,
                        (true, true) => CompletionPolicy::EscapedProcess,
                        (true, false) => CompletionPolicy::FailedWatch,
                    };
                    self.active_watches.insert(
                        game.id,
                        ActiveWatch {
                            policy,
                            job_released: false,
                            target_exited: false,
                            terminal_sent: false,
                            _target: target,
                            _plugin_activity: prepared_plugin.take(),
                            _generic_activity: generic_activity.take(),
                            plugin_cancel: None,
                            plugin_root: None,
                            runtime_thread: None,
                            label: game.label.clone(),
                            close_script: game.close_script.clone(),
                            watched: watch_target.clone().map(|name| WatchedProgram {
                                name,
                                path: watch_path.clone(),
                            }),
                            program_watch: None,
                        },
                    );
                    WorkerResult::Launched {
                        game_id: game.id,
                        root_pid: spawned.pid,
                        root_created: spawned.created,
                        updated_config,
                    }
                }
                Err(message) => WorkerResult::Error {
                    title: t!("errors.launch_failed", game => game.label).to_string(),
                    message,
                },
            }
        }))
    }
}

impl WorkerContext {
    pub(super) fn stop_if_cancelled(&mut self, game: GameId, cancel: &LaunchCancel) -> bool {
        if !cancel.is_cancelled() {
            return false;
        }
        if let Err(error) = self.processes.stop_game(game) {
            self.log.event(
                "launch-cancel-stop-error",
                &[("game_id", game.to_string()), ("error", error)],
            );
        }
        true
    }
}

pub(super) fn reconcile_request(game: &GameEntry) -> Result<ReconcileRequest, String> {
    let assignment = game
        .plugin
        .as_ref()
        .ok_or_else(|| t!("errors.no_plugin_assignment").to_string())?;
    let policy = match assignment.update_policy {
        PluginUpdatePolicy::TrackLatest => RevisionPolicy::TrackLatest {
            allow_prerelease: assignment.allow_prerelease,
        },
        PluginUpdatePolicy::Pinned => RevisionPolicy::Pinned {
            version: assignment.revision.version.clone(),
            sha256: assignment.revision.sha256.clone(),
        },
    };
    Ok(ReconcileRequest {
        assignment_id: assignment.assignment_id.clone(),
        plugin_id: assignment.plugin_id.clone(),
        game_id: game.id.0,
        selected_target: game.selected_target.clone(),
        policy,
        choices: ResolutionChoices {
            launcher_destination: assignment.resolved_launcher.clone(),
            watch_destination: assignment.resolved_watch_target.clone(),
            external_grants: assignment.grants.clone(),
            ..ResolutionChoices::default()
        },
        copy_zero_matches_to_game_folder: assignment.copy_zero_matches_to_game_folder,
        search_directory_destinations: assignment.search_directory_destinations,
        modification_decisions: BTreeMap::<String, ModificationDecision>::new(),
    })
}

pub(super) fn apply_reconciliation(
    game: &mut GameEntry,
    result: &ReconcileResult,
) -> Result<bool, String> {
    let previous_config_targets = game.config_targets.clone();
    let assignment = game
        .plugin
        .as_mut()
        .ok_or_else(|| t!("errors.reconciliation_unassigned").to_string())?;
    if assignment.plugin_id != result.revision.plugin_id
        || assignment.assignment_id != result.ledger.assignment_id
    {
        return Err(t!("errors.reconciliation_mismatch").to_string());
    }
    let previous_assignment = assignment.clone();
    assignment.revision = PluginRevision {
        version: result.revision.plugin_version.clone(),
        sha256: result.revision.package_sha256.clone(),
    };
    assignment.resolved_launcher = result.ledger.resolved_launcher.clone();
    assignment.resolved_watch_target = result.ledger.resolved_watch_target.clone();
    assignment.ledger_reference = result.ledger_reference.clone();
    game.config_targets = result.config_targets.clone();
    Ok(*assignment != previous_assignment || game.config_targets != previous_config_targets)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/launch__tests.rs"]
mod tests;
