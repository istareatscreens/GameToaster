use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::{WorkerCommand, WorkerResult};
use super::plugin_runtime::{forward_runtime_events, run_runtime_operation, runtime_spec};
use super::sessions::{ActiveWatch, CompletionPolicy};
use crate::launcher_gui::config;
use crate::launcher_gui::model::{GameEntry, LauncherConfig, PluginRevision, PluginUpdatePolicy};
use crate::plugin::deployment::ModificationDecision;
use crate::plugin::lifecycle::{ReconcileRequest, ReconcileResult, RevisionPolicy};
use crate::plugin::manifest::Operation as ManifestOperation;
use crate::plugin::paths::ResolutionChoices;
use crate::plugin::runtime::{Operation as RuntimeOperation, RuntimeHost};

impl WorkerContext {
    pub(super) fn launch(
        &mut self,
        mut game: GameEntry,
        mut config: LauncherConfig,
    ) -> WorkerResult {
        let console = config.settings.console();
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
            WorkerResult::Error {
                title: t!("errors.prepare_failed", game => game.label).to_string(),
                message,
            }
        } else if let Some(spec) = runtime_launch {
            match RuntimeHost::start(spec) {
                Ok(handle) => {
                    let root_pid = handle.process_id;
                    let root_created = handle.created;
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
                                    runtime_thread: Some(runtime_thread),
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
                            runtime_thread: None,
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
        }
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
