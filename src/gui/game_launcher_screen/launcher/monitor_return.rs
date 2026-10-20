use std::time::{Duration, Instant};

use super::super::display::monitor_watch::{MonitorTarget, MonitorWatchEvent};
use super::super::lifecycle;
use super::super::ControllerScreenConfig;
use super::status::ControllerStatus;
use super::ControllerLauncher;

const RETURN_RETRY_STEP: Duration = Duration::from_secs(1);
const RETURN_RETRY_MAX: Duration = Duration::from_secs(15);
const RETURN_RETRY_ATTEMPTS: u32 = 40;

#[derive(Clone, Copy, Debug)]
pub(super) struct ReturnRetry {
    attempts: u32,
    next_attempt: Instant,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) enum MonitorReturnState {
    #[default]
    Inactive,
    Waiting(MonitorTarget),
    Pending(MonitorTarget),
}

impl MonitorReturnState {
    pub(super) fn target(&self) -> Option<&MonitorTarget> {
        match self {
            Self::Inactive => None,
            Self::Waiting(target) | Self::Pending(target) => Some(target),
        }
    }
}

impl ControllerLauncher {
    pub(crate) fn monitor_return_configured(config: &ControllerScreenConfig) -> bool {
        MonitorTarget::from_config(config).is_some()
    }

    pub(crate) fn arm_monitor_return(&mut self, config: &ControllerScreenConfig) -> bool {
        match MonitorTarget::from_config(config) {
            Some(target) => {
                if self.monitor_return == MonitorReturnState::Inactive {
                    self.monitor_return = MonitorReturnState::Waiting(target);
                }
                self.diagnostics
                    .transition("monitor-return", "armed-os-startup");
                true
            }
            None => false,
        }
    }

    #[cfg(test)]
    fn monitor_return_is_armed(&self) -> bool {
        self.monitor_return != MonitorReturnState::Inactive
    }

    pub(super) fn poll_monitor_watcher(&mut self, config: &ControllerScreenConfig) {
        for event in self.monitor_watcher.poll() {
            self.handle_monitor_watch_event(event, config);
        }
        if let Some(target) = MonitorTarget::from_config(config) {
            if let Some(error) = self.monitor_watcher.probe_error(&target) {
                self.diagnostics.error("monitor-watch", &error);
            }
        }

        if let MonitorReturnState::Waiting(target) = &self.monitor_return {
            if self.monitor_watcher.presence_of(target) == Some(true) {
                let target = target.clone();
                self.monitor_return = MonitorReturnState::Pending(target);
                self.return_retry = Some(ReturnRetry {
                    attempts: 0,
                    next_attempt: Instant::now() + RETURN_RETRY_STEP,
                });
                self.ctx.request_repaint_after(RETURN_RETRY_STEP);
                self.diagnostics
                    .transition("monitor-return", "present-while-waiting;pending-open");
            }
        }
        self.complete_monitor_return_at(config, Instant::now());
    }

    fn handle_monitor_watch_event(
        &mut self,
        event: MonitorWatchEvent,
        config: &ControllerScreenConfig,
    ) {
        let configured_target = MonitorTarget::from_config(config);
        match event {
            MonitorWatchEvent::Unavailable(target)
                if configured_target.as_ref() == Some(&target) =>
            {
                if self.open {
                    if let Err(error) = self.close() {
                        self.diagnostics.error("monitor-unavailable", &error);
                        self.status = ControllerStatus::Error(error);
                    }
                }
                self.monitor_return = MonitorReturnState::Waiting(target);
                self.return_retry = None;
                self.diagnostics
                    .transition("monitor-unavailable", "armed=true");
            }
            MonitorWatchEvent::Returned(target) if configured_target.as_ref() == Some(&target) => {
                self.monitor_return = MonitorReturnState::Pending(target);
                self.return_retry = None;
                self.diagnostics
                    .transition("monitor-returned", "pending-open");
            }
            MonitorWatchEvent::Unavailable(_) | MonitorWatchEvent::Returned(_) => {
                if configured_target.as_ref() != self.monitor_return.target() {
                    self.monitor_return = MonitorReturnState::Inactive;
                    self.return_retry = None;
                    self.diagnostics
                        .transition("monitor-event-ignored", "stale-target");
                }
            }
        }
    }

    #[cfg(test)]
    fn complete_monitor_return(&mut self, config: &ControllerScreenConfig) {
        self.complete_monitor_return_at(config, Instant::now());
    }

    fn complete_monitor_return_at(&mut self, config: &ControllerScreenConfig, now: Instant) {
        let MonitorReturnState::Pending(target) = &self.monitor_return else {
            return;
        };
        let target = target.clone();
        if MonitorTarget::from_config(config).as_ref() != Some(&target) {
            self.monitor_return = MonitorReturnState::Inactive;
            self.return_retry = None;
            return;
        }
        if self.lifecycle.state() != lifecycle::coordinator::State::Idle {
            return;
        }
        if self.open {
            self.monitor_return = MonitorReturnState::Inactive;
            self.return_retry = None;
            return;
        }
        if let Some(retry) = self.return_retry {
            if now < retry.next_attempt {
                self.ctx.request_repaint_after(retry.next_attempt - now);
                return;
            }
        }
        match self.open(config) {
            Ok(()) => {
                self.return_retry = None;
                self.diagnostics
                    .transition("monitor-returned", "launcher-opened");
            }
            Err(error) => {
                let attempts = self.return_retry.map_or(0, |retry| retry.attempts) + 1;
                if attempts >= RETURN_RETRY_ATTEMPTS {
                    self.return_retry = None;
                    self.monitor_return = MonitorReturnState::Inactive;
                    self.diagnostics.error(
                        "monitor-returned",
                        &format!(
                            "gave up after {attempts} attempts; the next display return re-arms: {}",
                            error.logged
                        ),
                    );
                } else {
                    let backoff = (RETURN_RETRY_STEP * attempts).min(RETURN_RETRY_MAX);
                    self.return_retry = Some(ReturnRetry {
                        attempts,
                        next_attempt: now + backoff,
                    });

                    self.monitor_return = MonitorReturnState::Pending(target);
                    self.diagnostics.transition(
                        "monitor-returned",
                        format!(
                            "retry;attempt={attempts};backoff_ms={};error={}",
                            backoff.as_millis(),
                            error.logged
                        ),
                    );
                    self.ctx.request_repaint_after(backoff);
                }
                self.status = ControllerStatus::DisplayChanged(error.shown);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/monitor_return__tests.rs"]
mod tests;
