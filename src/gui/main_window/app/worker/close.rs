use std::thread;
use std::time::Duration;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use crate::launcher_gui::model::{CloseMode, ClosePlan, GameEntry};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionClosed {
    None,

    PluginCancelled,

    Removed,
}

fn exit_observed(mode: CloseMode, session: SessionClosed) -> bool {
    mode == CloseMode::KillExe && session == SessionClosed::Removed
}

impl WorkerContext {
    pub(super) fn close_game(&mut self, game: GameEntry) -> WorkerResult {
        let outcome = match game.close_mode {
            CloseMode::Script => match &game.close_script {
                Some(script) => self.runtime.run_close_script(script),
                None => Err(t!("errors.no_close_script").to_string()),
            },
            CloseMode::KillExe => {
                let stopped = self.processes.stop_game(game.id);

                match game.watch_target() {
                    Some(watch) => {
                        let watch_path = game.watch_target_path();
                        match watch_path.as_deref() {
                            Some(path) => crate::launcher_gui::process::terminate_by_path(path),
                            None => crate::launcher_gui::process::terminate_by_name(&watch),
                        };

                        let mut gone = false;
                        for _ in 0..20 {
                            let running = watch_path
                                .as_deref()
                                .map(crate::launcher_gui::process::is_process_path_running)
                                .unwrap_or_else(|| {
                                    crate::launcher_gui::process::is_process_running(&watch)
                                });
                            if !running {
                                gone = true;
                                break;
                            }
                            thread::sleep(Duration::from_millis(100));
                        }
                        if gone {
                            stopped
                        } else {
                            Err(t!("errors.still_running", exe => watch).to_string())
                        }
                    }
                    None => stopped,
                }
            }
        };
        match outcome {
            Ok(()) => {
                let mut session = SessionClosed::None;
                if let Some(watch) = self.active_watches.get_mut(&game.id) {
                    if let Some(cancel) = &watch.plugin_cancel {
                        let _ = cancel.cancel();
                        session = SessionClosed::PluginCancelled;
                    } else {
                        self.active_watches.remove(&game.id);
                        session = SessionClosed::Removed;
                    }
                }
                WorkerResult::Closed {
                    game_id: game.id,
                    exit_observed: exit_observed(game.close_mode, session),
                }
            }
            Err(message) => WorkerResult::Error {
                title: t!("errors.close_game_failed", game => game.label).to_string(),
                message,
            },
        }
    }

    pub(super) fn close_active_games(&self, games: Vec<GameEntry>) -> WorkerResult {
        for game in &games {
            match game.close_plan() {
                ClosePlan::RunScript(script) => {
                    if let Err(error) = self.runtime.run_close_script(&script) {
                        self.log.event(
                            "close-active-script-error",
                            &[("game_id", game.id.to_string()), ("error", error)],
                        );
                    }
                }
                ClosePlan::Kill(name) => {
                    crate::launcher_gui::process::terminate_by_name(&name);
                }

                ClosePlan::Nothing => self.log.event(
                    "close-active-noop",
                    &[
                        ("game_id", game.id.to_string()),
                        ("close_mode", format!("{:?}", game.close_mode)),
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
