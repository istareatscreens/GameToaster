use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::{ControllerLights, LightsError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct LightsTiming {
    pub settle: Duration,

    pub retry_every: Duration,

    pub give_up_after: Duration,
}

pub(in crate::gui::game_launcher_screen) const TIMING: LightsTiming = LightsTiming {
    settle: Duration::from_millis(500),
    retry_every: Duration::from_millis(250),
    give_up_after: Duration::from_secs(15),
};

const CANCEL_POLL: Duration = Duration::from_millis(20);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum LightsOutcome {
    TurnedOff { attempts: u32 },

    Cancelled,

    GaveUp { attempts: u32 },
    Failed(String),
}

struct Running {
    cancel: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct LightsOffJob {
    running: Option<Running>,
}

impl LightsOffJob {
    pub fn start(
        &mut self,
        lights: &'static dyn ControllerLights,
        ports: String,
        timing: LightsTiming,
        report: impl FnOnce(LightsOutcome) + Send + 'static,
    ) -> Result<bool, String> {
        if self.is_running() {
            return Ok(false);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let thread = thread::Builder::new()
            .name("gametoaster-lights-off".into())
            .spawn(move || report(run(lights, &ports, timing, &flag)))
            .map_err(|error| format!("failed to start the lights-off thread: {error}"))?;
        self.running = Some(Running { cancel, thread });
        Ok(true)
    }

    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| !running.thread.is_finished())
    }
}

impl Drop for LightsOffJob {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn run(
    lights: &dyn ControllerLights,
    ports: &str,
    timing: LightsTiming,
    cancel: &AtomicBool,
) -> LightsOutcome {
    if wait(timing.settle, cancel) {
        return LightsOutcome::Cancelled;
    }
    let deadline = Instant::now() + timing.give_up_after;
    let mut attempts = 0;
    loop {
        if cancel.load(Ordering::Acquire) {
            return LightsOutcome::Cancelled;
        }
        attempts += 1;
        match lights.turn_off(ports) {
            Ok(()) => return LightsOutcome::TurnedOff { attempts },
            Err(LightsError::Failed(message)) => return LightsOutcome::Failed(message),
            Err(LightsError::Busy) => {}
        }
        if Instant::now() + timing.retry_every > deadline {
            return LightsOutcome::GaveUp { attempts };
        }
        if wait(timing.retry_every, cancel) {
            return LightsOutcome::Cancelled;
        }
    }
}

fn wait(duration: Duration, cancel: &AtomicBool) -> bool {
    let until = Instant::now() + duration;
    loop {
        if cancel.load(Ordering::Acquire) {
            return true;
        }
        let now = Instant::now();
        if now >= until {
            return false;
        }
        thread::sleep(CANCEL_POLL.min(until - now));
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/controller/framework/lights/job__tests.rs"]
mod tests;
