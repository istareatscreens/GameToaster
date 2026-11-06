use std::path::PathBuf;
use std::time::{Duration, Instant};

use rust_i18n::t;

use crate::launcher_gui::dialogs::Dialog;
use crate::launcher_gui::model::GameId;

const WINDOW_RENDER_TIMEOUT: Duration = Duration::from_secs(30);
const APPEAR_TIMEOUT: Duration = Duration::from_secs(45);

const FAILED_LOOK_INTERVAL: Duration = Duration::from_secs(1);

pub(super) const GONE_AFTER: Duration = Duration::from_secs(30);

pub(super) enum HideTick {
    Wait,
    Confirmed,
    Failed(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::main_window::app) enum Presence {
    Gone,

    Back,
}

pub(super) fn reports_gone(dialog: Option<&Dialog>) -> bool {
    matches!(dialog, Some(Dialog::Error { title, .. }) if *title == t!("errors.game_no_longer_running"))
}

pub(super) struct GoneTracker {
    after: Duration,
    seen: bool,
    gone_since: Option<Instant>,
    reported: bool,
}

impl GoneTracker {
    pub(super) fn new(after: Duration) -> Self {
        Self {
            after,
            seen: false,
            gone_since: None,
            reported: false,
        }
    }

    pub(super) fn observe(&mut self, running: bool, now: Instant) -> Option<Presence> {
        if running {
            self.seen = true;
            self.gone_since = None;
            return std::mem::take(&mut self.reported).then_some(Presence::Back);
        }
        if !self.seen {
            return None;
        }
        let since = *self.gone_since.get_or_insert(now);
        if !self.reported && now.saturating_duration_since(since) >= self.after {
            self.reported = true;
            return Some(Presence::Gone);
        }
        None
    }
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

    failed: Option<String>,

    next_look: Instant,

    plugin: bool,
}

impl PendingHide {
    pub(super) fn new(game_id: GameId, watch_exe: String, watch_path: Option<PathBuf>) -> Self {
        Self {
            game_id,
            watch_exe,
            watch_path,
            appear_deadline: Instant::now() + APPEAR_TIMEOUT,
            confirming: None,
            failed: None,
            next_look: Instant::now(),
            plugin: false,
        }
    }

    pub(super) fn for_plugin(mut self) -> Self {
        self.plugin = true;
        self
    }

    pub(super) fn follows_after_confirmation(&self) -> bool {
        self.plugin
    }

    pub(super) fn status(&self) -> String {
        self.failed
            .clone()
            .unwrap_or_else(|| t!("status.loading", exe => self.watch_exe.as_str()).to_string())
    }

    pub(super) fn has_failed(&self) -> bool {
        self.failed.is_some()
    }

    pub(super) fn reported_by(&self, dialog: Option<&Dialog>) -> bool {
        matches!(
            (self.failed.as_deref(), dialog),
            (Some(failure), Some(Dialog::Error { message, .. })) if message == failure
        )
    }

    fn fail(&mut self, message: String) -> HideTick {
        self.failed = Some(message.clone());
        self.next_look = Instant::now() + FAILED_LOOK_INTERVAL;
        HideTick::Failed(message)
    }

    pub(super) fn tick(&mut self) -> HideTick {
        use crate::launcher_gui::process;
        if self.confirming.is_none() {
            if self.failed.is_some() {
                if Instant::now() < self.next_look {
                    return HideTick::Wait;
                }
                self.next_look = Instant::now() + FAILED_LOOK_INTERVAL;
            }
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
            if self.failed.is_some() || Instant::now() < self.appear_deadline {
                return HideTick::Wait;
            }
            let message = t!("errors.launch_timed_out", exe => self.watch_exe.as_str()).to_string();
            return self.fail(message);
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
                return HideTick::Confirmed;
            }
            self.confirming = None;
            if self.failed.is_some() || Instant::now() < self.appear_deadline {
                return HideTick::Wait;
            }
            let message = t!(
                "errors.exited_immediately",
                exe => self.watch_exe.as_str(),
                code => format!("{code:08X}")
            )
            .to_string();
            self.fail(message)
        } else if process::has_visible_window(pid)
            || (!self.plugin && Instant::now() >= render_deadline)
        {
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
