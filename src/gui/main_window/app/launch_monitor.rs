use std::path::PathBuf;
use std::time::{Duration, Instant};

use rust_i18n::t;

use crate::launcher_gui::model::GameId;

const WINDOW_RENDER_TIMEOUT: Duration = Duration::from_secs(30);
const APPEAR_TIMEOUT: Duration = Duration::from_secs(45);

pub(super) enum HideTick {
    Wait,
    Confirmed,
    Failed(String),
}

pub(super) struct PendingHide {
    pub(super) game_id: GameId,
    pub(super) watch_exe: String,
    watch_path: Option<PathBuf>,
    appear_deadline: Instant,
    confirming: Option<(
        u32,
        Option<crate::launcher_gui::process::ProcessWatch>,
        Instant,
    )>,
}

impl PendingHide {
    pub(super) fn new(game_id: GameId, watch_exe: String, watch_path: Option<PathBuf>) -> Self {
        Self {
            game_id,
            watch_exe,
            watch_path,
            appear_deadline: Instant::now() + APPEAR_TIMEOUT,
            confirming: None,
        }
    }

    pub(super) fn tick(&mut self) -> HideTick {
        use crate::launcher_gui::process;
        if self.confirming.is_none() {
            let pid = self
                .watch_path
                .as_deref()
                .and_then(process::find_pid_by_path)
                .or_else(|| {
                    self.watch_path
                        .is_none()
                        .then(|| process::find_pid_by_name(&self.watch_exe))
                        .flatten()
                });
            if let Some(pid) = pid {
                self.confirming = Some((
                    pid,
                    process::ProcessWatch::open(pid),
                    Instant::now() + WINDOW_RENDER_TIMEOUT,
                ));
                return HideTick::Wait;
            }
            return if Instant::now() >= self.appear_deadline {
                HideTick::Failed(
                    t!("errors.launch_timed_out", exe => self.watch_exe.as_str()).to_string(),
                )
            } else {
                HideTick::Wait
            };
        }

        let (exit, pid, render_deadline) = {
            let Some((pid, watch, render_deadline)) = self.confirming.as_ref() else {
                return HideTick::Wait;
            };
            let exit = match watch {
                Some(handle) => handle.exit_code(),
                None => (!self.is_running()).then_some(0),
            };
            (exit, *pid, *render_deadline)
        };
        if let Some(code) = exit {
            if self.is_running() {
                HideTick::Confirmed
            } else if Instant::now() < self.appear_deadline {
                self.confirming = None;
                HideTick::Wait
            } else {
                HideTick::Failed(
                    t!(
                        "errors.exited_immediately",
                        exe => self.watch_exe.as_str(),
                        code => format!("{code:08X}")
                    )
                    .to_string(),
                )
            }
        } else if process::has_visible_window(pid) || Instant::now() >= render_deadline {
            HideTick::Confirmed
        } else {
            HideTick::Wait
        }
    }

    fn is_running(&self) -> bool {
        self.watch_path
            .as_deref()
            .map(crate::launcher_gui::process::is_process_path_running)
            .unwrap_or_else(|| crate::launcher_gui::process::is_process_running(&self.watch_exe))
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/launch_monitor__tests.rs"]
mod tests;
