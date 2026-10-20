use std::time::{Duration, Instant};

use super::super::controller::RELEASE_TIMEOUT;
use super::super::display::monitor_watch::MonitorTarget;
use super::super::{describe_monitors, ControllerScreenConfig};
use super::monitor_return::MonitorReturnState;
use super::resolved_display::resolve_display;
use super::status::ControllerStatus;
use super::ControllerLauncher;
use crate::gui::common::message::Message;

const RECONNECT_SETTLE: Duration = Duration::from_millis(200);

const RECONNECT_BACKOFF_STEP: Duration = Duration::from_millis(400);
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_millis(2000);

const RECONNECT_MAX_ATTEMPTS: u32 = 24;

#[derive(Clone, Copy, Debug)]
pub(super) struct ReconnectState {
    attempts: u32,
    next_attempt: Instant,
}

impl ControllerLauncher {
    pub(crate) fn is_reconnecting(&self) -> bool {
        self.reconnect.is_some()
    }

    pub(crate) fn open(&mut self, config: &ControllerScreenConfig) -> Result<(), Message> {
        let shape = self.layouts.surface_shape(&config.launcher_layout);
        let display = match resolve_display(config, shape) {
            Ok(display) => display,
            Err(error) => {
                if let Some(target) = MonitorTarget::from_config(config) {
                    self.monitor_return = MonitorReturnState::Waiting(target);
                }

                self.diagnostics.error("viewport-open", &error.logged);
                let saved = config
                    .calibration
                    .as_ref()
                    .map(|calibration| calibration.monitor_id.clone());
                match describe_monitors(saved.as_deref()) {
                    Ok(lines) => {
                        for line in lines {
                            self.diagnostics.transition("viewport-open-display", line);
                        }
                    }
                    Err(enumerate_error) => self
                        .diagnostics
                        .transition("viewport-open-display", format!("error={enumerate_error}")),
                }
                return Err(error);
            }
        };
        self.layouts.activate(&config.launcher_layout)?;
        self.diagnostics.transition(
            "viewport-open",
            format!(
                "monitor={}x{}@{},{};surface={},{}+{}x{}",
                display.monitor.bounds.width,
                display.monitor.bounds.height,
                display.monitor.bounds.x,
                display.monitor.bounds.y,
                display.surface.x,
                display.surface.y,
                display.surface.width,
                display.surface.height,
            ),
        );
        self.display = Some(display);
        self.open = true;
        self.suspended = false;

        self.force_display_refresh = true;
        self.active_controller_config = Some(config.controller.clone());
        self.last_display_refresh = Instant::now();
        self.test_transient = false;
        self.monitor_return = MonitorReturnState::Inactive;
        self.return_retry = None;

        self.begin_connect(RECONNECT_SETTLE);
        Ok(())
    }

    fn begin_connect(&mut self, initial_delay: Duration) {
        self.reconnect = Some(ReconnectState {
            attempts: 0,
            next_attempt: Instant::now() + initial_delay,
        });
        self.status = ControllerStatus::Connecting;
    }

    pub(super) fn tick_connect(&mut self) {
        let Some(mut state) = self.reconnect else {
            return;
        };
        if Instant::now() < state.next_attempt {
            return;
        }
        let Some(config) = self.active_controller_config.clone() else {
            self.reconnect = None;
            return;
        };
        match self.manager.connect(&config) {
            Ok(()) => {
                self.reconnect = None;

                self.status = ControllerStatus::Connecting;
                self.diagnostics.transition(
                    "reconnect",
                    format!("opened;attempt={}", state.attempts + 1),
                );
            }
            Err(error) => {
                state.attempts += 1;
                if state.attempts >= RECONNECT_MAX_ATTEMPTS {
                    self.reconnect = None;
                    self.diagnostics.error(
                        "reconnect",
                        &format!("gave up after {} attempts: {error}", state.attempts),
                    );
                    self.status = ControllerStatus::ConnectError(error);
                } else {
                    let backoff =
                        (RECONNECT_BACKOFF_STEP * state.attempts).min(RECONNECT_BACKOFF_MAX);
                    state.next_attempt = Instant::now() + backoff;
                    self.reconnect = Some(state);
                    self.status = ControllerStatus::Reconnecting;
                    self.diagnostics.transition(
                        "reconnect",
                        format!(
                            "retry;attempt={};backoff_ms={}",
                            state.attempts,
                            backoff.as_millis()
                        ),
                    );
                }
            }
        }
    }

    pub(crate) fn close(&mut self) -> Result<(), String> {
        self.open = false;
        self.suspended = false;
        self.reconnect = None;
        self.placement = None;
        self.monitor_return = MonitorReturnState::Inactive;
        self.return_retry = None;

        self.layouts.deactivate();
        self.manager.disconnect(RELEASE_TIMEOUT)?;
        self.active_controller_config = None;
        self.status = ControllerStatus::Disconnected;
        self.diagnostics
            .transition("viewport-close", "controller-released");
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/session__tests.rs"]
mod tests;
