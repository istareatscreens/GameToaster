use std::thread::JoinHandle;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use super::plugin_runtime::log_runtime_event;
use crate::launcher_gui::model::GameId;
use crate::launcher_gui::process::ProcessCompletion;
use crate::plugin::lifecycle::{ManagedGameActivity, PreparedLaunch};
use crate::plugin::runtime::{OperationCancel, RuntimeEvent};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CompletionPolicy {
    Job,

    EscapedProcess,

    FailedWatch,

    PluginRuntime,
}

pub(super) struct ActiveWatch {
    pub(super) policy: CompletionPolicy,

    pub(super) job_released: bool,

    pub(super) target_exited: bool,

    pub(super) terminal_sent: bool,
    pub(super) _target: Option<crate::launcher_gui::process::EscapedProcessWatch>,
    pub(super) _plugin_activity: Option<PreparedLaunch>,
    pub(super) _generic_activity: Option<ManagedGameActivity>,
    pub(super) plugin_cancel: Option<OperationCancel>,
    pub(super) runtime_thread: Option<JoinHandle<()>>,
}

impl ActiveWatch {
    fn is_terminal(&self) -> bool {
        match self.policy {
            CompletionPolicy::Job => self.job_released,
            CompletionPolicy::EscapedProcess => self.target_exited,
            CompletionPolicy::FailedWatch => false,
            CompletionPolicy::PluginRuntime => self.target_exited,
        }
    }
}

impl WorkerContext {
    pub(super) fn process_completion(&mut self, completion: ProcessCompletion) -> WorkerResult {
        match completion {
            ProcessCompletion::Exited(game_id) => {
                if self.processes.release_completed(game_id) {
                    if let Some(watch) = self.active_watches.get_mut(&game_id) {
                        watch.job_released = true;
                    }
                }
                WorkerResult::Internal
            }
            ProcessCompletion::Failed { message } => WorkerResult::Error {
                title: t!("errors.completion_watch_failed").to_string(),
                message,
            },
        }
    }

    pub(super) fn watch_event(
        &mut self,
        event: crate::launcher_gui::process::WatchEvent,
    ) -> WorkerResult {
        use crate::launcher_gui::process::WatchEvent;
        let game_id = match &event {
            WatchEvent::Attached { game_id, .. }
            | WatchEvent::Exited { game_id, .. }
            | WatchEvent::NotFound { game_id }
            | WatchEvent::Error { game_id, .. } => *game_id,
        };
        match event {
            WatchEvent::Attached { .. } => {}
            WatchEvent::Exited { .. } => {
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.target_exited = true;
                }
            }
            WatchEvent::NotFound { .. } => {
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.policy = CompletionPolicy::FailedWatch;
                }
                self.log.event(
                    "process-watch-not-found",
                    &[("game_id", game_id.to_string())],
                );
            }
            WatchEvent::Error { message, .. } => {
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.policy = CompletionPolicy::FailedWatch;
                }
                self.log.event(
                    "process-watch-error",
                    &[("game_id", game_id.to_string()), ("error", message)],
                );
            }
        }
        WorkerResult::Internal
    }

    pub(super) fn plugin_runtime_event(
        &mut self,
        game_id: GameId,
        operation_id: String,
        event: RuntimeEvent,
    ) -> WorkerResult {
        log_runtime_event(&self.log, game_id, &operation_id, &event);
        match event {
            RuntimeEvent::Started { .. }
            | RuntimeEvent::Connected { .. }
            | RuntimeEvent::Invalid(_)
            | RuntimeEvent::Ignored { .. } => WorkerResult::Internal,
            RuntimeEvent::Message(message) => {
                if let crate::plugin::runtime::Message::GameState(state) = &message {
                    self.game_state.apply(state);
                }
                WorkerResult::PluginMessage { game_id, message }
            }
            RuntimeEvent::Exited(child) => {
                self.game_state.clear();
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.target_exited = true;
                }
                let end = if child.succeeded() {
                    super::super::runtime_end::RuntimeEnd::Clean
                } else {
                    let mut message =
                        t!("plugins.error.process_exit", code => child.code).to_string();
                    if let Some(reason) = child.reason() {
                        message.push_str(": ");
                        message.push_str(&reason);
                    }
                    super::super::runtime_end::RuntimeEnd::Failed {
                        title: t!("plugins.error.runtime_launch").to_string(),
                        message,
                    }
                };
                WorkerResult::PluginRuntimeEnded { game_id, end }
            }
            RuntimeEvent::TimedOut => {
                self.game_state.clear();
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.target_exited = true;
                }
                WorkerResult::PluginRuntimeEnded {
                    game_id,
                    end: super::super::runtime_end::RuntimeEnd::Failed {
                        title: t!("plugins.error.runtime_stopped").to_string(),
                        message: t!("plugins.error.runtime_timeout").to_string(),
                    },
                }
            }
            RuntimeEvent::LauncherCancelled => {
                self.game_state.clear();
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.target_exited = true;
                }

                WorkerResult::PluginRuntimeEnded {
                    game_id,
                    end: super::super::runtime_end::RuntimeEnd::Cancelled,
                }
            }
            RuntimeEvent::WorkerFailure(message) => {
                self.game_state.clear();
                if let Some(watch) = self.active_watches.get_mut(&game_id) {
                    watch.target_exited = true;
                }
                WorkerResult::PluginRuntimeEnded {
                    game_id,
                    end: super::super::runtime_end::RuntimeEnd::Failed {
                        title: t!("plugins.error.runtime_stopped").to_string(),
                        message,
                    },
                }
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        for watch in self.active_watches.values() {
            if let Some(cancel) = &watch.plugin_cancel {
                let _ = cancel.cancel();
            }
        }
        let watches = std::mem::take(&mut self.active_watches);
        for (_, mut watch) in watches {
            if let Some(runtime_thread) = watch.runtime_thread.take() {
                let _ = runtime_thread.join();
            }
        }
        self.game_state.clear();
        let result = self.processes.shutdown();
        let _ = self.results.send(WorkerResult::ShutdownComplete(result));

        self.ctx.request_repaint();
    }

    pub(super) fn emit_completed_games(&mut self) {
        let ready: Vec<GameId> = self
            .active_watches
            .iter_mut()
            .filter_map(|(game_id, watch)| {
                (watch.is_terminal() && !watch.terminal_sent).then(|| {
                    watch.terminal_sent = true;
                    *game_id
                })
            })
            .collect();
        for game_id in ready {
            if let Some(mut watch) = self.active_watches.remove(&game_id) {
                if let Some(runtime_thread) = watch.runtime_thread.take() {
                    let _ = runtime_thread.join();
                }
            }
            self.game_state.clear();
            if self
                .results
                .send(WorkerResult::GameExited(game_id))
                .is_err()
            {
                break;
            }

            self.ctx.request_repaint();
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/sessions__tests.rs"]
mod tests;
