use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::launcher_gui::logging::LauncherLog;

#[derive(Debug)]
pub(super) struct RateLimiter {
    interval: Duration,
    last: BTreeMap<&'static str, Instant>,
    suppressed: BTreeMap<&'static str, u64>,
}

impl RateLimiter {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: BTreeMap::new(),
            suppressed: BTreeMap::new(),
        }
    }

    pub fn admit(&mut self, key: &'static str, now: Instant) -> Option<u64> {
        if self
            .last
            .get(key)
            .is_some_and(|last| now.duration_since(*last) < self.interval)
        {
            *self.suppressed.entry(key).or_default() += 1;
            return None;
        }
        self.last.insert(key, now);
        Some(self.suppressed.remove(key).unwrap_or(0))
    }
}

#[cfg(test)]
pub(in crate::gui::game_launcher_screen) fn redact_device_path(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.starts_with(r"\\?\hid#") || lower.starts_with(r"\\.\") {
        format!("device:{:016x}", stable_hash(path.as_bytes()))
    } else {
        path.to_string()
    }
}

#[cfg(test)]
fn stable_hash(bytes: &[u8]) -> u64 {
    crate::gui::common::hash::fnv1a_64(bytes)
}

pub(in crate::gui::game_launcher_screen) struct ControllerDiagnostics {
    log: LauncherLog,
    limiter: RateLimiter,
}

impl ControllerDiagnostics {
    pub fn new(log: LauncherLog) -> Self {
        Self {
            log,
            limiter: RateLimiter::new(Duration::from_secs(2)),
        }
    }

    pub fn log(&self) -> LauncherLog {
        self.log.clone()
    }

    pub fn transition(&self, transition: &str, detail: impl Into<String>) {
        self.log.event(
            "game-launcher-screen-transition",
            &[("transition", transition.into()), ("detail", detail.into())],
        );
    }

    pub fn error(&mut self, key: &'static str, message: &str) {
        if let Some(suppressed) = self.limiter.admit(key, Instant::now()) {
            self.log.event(
                "game-launcher-screen-error",
                &[
                    ("category", key.into()),
                    ("message", message.into()),
                    ("suppressed_since_last", suppressed.to_string()),
                ],
            );
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/support/diagnostics__tests.rs"]
mod tests;
