use std::thread;
use std::time::Duration;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use crate::launcher_gui::model::{ClosePlan, GameEntry, GameId};
use crate::support::process::tree::end_process_tree;

const END_WAIT_MS: u32 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionClosed {
    None,

    PluginCancelled,

    Removed,
}

fn exit_observed(session: SessionClosed) -> bool {
    session == SessionClosed::Removed
}

impl WorkerContext {
    pub(super) fn close_game(&mut self, game: GameEntry) -> WorkerResult {
        let plugin_cancelled = self.end_plugin(game.id);
        let outcome = self.kill_game(&game);
        match outcome {
            Ok(ended) => {
                let mut session = SessionClosed::None;
                let mut session_close_script = None;
                if plugin_cancelled {
                    session = SessionClosed::PluginCancelled;
                } else if let Some(watch) = self.active_watches.remove(&game.id) {
                    session_close_script = watch.close_script;
                    session = SessionClosed::Removed;
                }

                let close_script = match session {
                    SessionClosed::Removed => session_close_script,
                    SessionClosed::None if ended > 0 => game.close_script.clone(),
                    SessionClosed::None | SessionClosed::PluginCancelled => None,
                };
                if let Some(script) = close_script {
                    if let Err(message) = self.start_close_script(&script) {
                        self.report(WorkerResult::Error {
                            title: t!("errors.close_script_failed", game => game.label).to_string(),
                            message,
                        });
                    }
                }
                WorkerResult::Closed {
                    game_id: game.id,
                    exit_observed: exit_observed(session),
                }
            }
            Err(message) => WorkerResult::Error {
                title: t!("errors.close_game_failed", game => game.label).to_string(),
                message,
            },
        }
    }

    fn end_plugin(&mut self, game: GameId) -> bool {
        let Some(watch) = self.active_watches.get(&game) else {
            return false;
        };
        let Some(cancel) = &watch.plugin_cancel else {
            return false;
        };
        if let Some((root, created)) = watch.plugin_root {
            end_process_tree(root, created, END_WAIT_MS);
        }
        let _ = cancel.cancel();
        true
    }

    fn kill_game(&mut self, game: &GameEntry) -> Result<usize, String> {
        let stopped = self.processes.stop_game(game.id);
        let Some(watch) = game.watch_target() else {
            return stopped.map(|()| 0);
        };
        let watch_path = game.watch_target_path();
        let ended = match watch_path.as_deref() {
            Some(path) => crate::launcher_gui::process::terminate_by_path(path),
            None => crate::launcher_gui::process::terminate_by_name(&watch),
        };

        let mut gone = false;
        for _ in 0..20 {
            let running = watch_path
                .as_deref()
                .map(crate::launcher_gui::process::is_process_path_running)
                .unwrap_or_else(|| crate::launcher_gui::process::is_process_running(&watch));
            if !running {
                gone = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        if gone {
            stopped.map(|()| ended)
        } else {
            Err(t!("errors.still_running", exe => watch).to_string())
        }
    }

    pub(super) fn close_active_games(&self, games: Vec<GameEntry>) -> WorkerResult {
        for game in &games {
            match game.close_plan() {
                ClosePlan::Kill(name) => {
                    let ended = crate::launcher_gui::process::terminate_by_name(&name);
                    if ended > 0 && !self.active_watches.contains_key(&game.id) {
                        if let Some(script) = &game.close_script {
                            if let Err(error) = self.start_close_script(script) {
                                self.log.event(
                                    "close-active-script-error",
                                    &[("game_id", game.id.to_string()), ("error", error)],
                                );
                            }
                        }
                    }
                }

                ClosePlan::Nothing => self.log.event(
                    "close-active-noop",
                    &[
                        ("game_id", game.id.to_string()),
                        (
                            "watch_exe",
                            game.watch_exe.clone().unwrap_or_else(|| "none".into()),
                        ),
                        ("executable", game.selected_target.display().to_string()),
                    ],
                ),
            }
        }
        WorkerResult::ActiveGamesClosed
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/close__tests.rs"]
mod tests;
