use std::sync::Mutex;
use std::time::{Duration, Instant};

use rust_i18n::t;
use windows::core::HSTRING;
use windows::ApplicationModel::Activation::ActivationKind;
use windows::ApplicationModel::{AppInstance, StartupTask, StartupTaskState};

pub(super) const TASK_ID: &str = "GameToasterStartup";

const STATE_REFRESH: Duration = Duration::from_secs(1);

static STATE: Mutex<StateMemo> = Mutex::new(StateMemo { last: None });

struct StateMemo {
    last: Option<(Instant, bool)>,
}

impl StateMemo {
    fn get(&mut self, now: Instant, query: impl FnOnce() -> bool) -> bool {
        if let Some((queried_at, enabled)) = self.last {
            if now.saturating_duration_since(queried_at) < STATE_REFRESH {
                return enabled;
            }
        }
        let enabled = query();
        self.last = Some((now, enabled));
        enabled
    }

    fn invalidate(&mut self) {
        self.last = None;
    }
}

fn task() -> Result<StartupTask, String> {
    StartupTask::GetAsync(&HSTRING::from(TASK_ID))
        .and_then(|operation| operation.get())
        .map_err(|error| format!("failed to open the startup task {TASK_ID}: {error}"))
}

fn starts_at_sign_in(state: StartupTaskState) -> bool {
    state == StartupTaskState::Enabled || state == StartupTaskState::EnabledByPolicy
}

fn enable_outcome(state: StartupTaskState) -> Result<(), String> {
    if starts_at_sign_in(state) {
        Ok(())
    } else if state == StartupTaskState::DisabledByUser {
        Err(t!("startup.task_disabled_by_user").to_string())
    } else if state == StartupTaskState::DisabledByPolicy {
        Err(t!("startup.task_disabled_by_policy").to_string())
    } else {
        Err(t!("startup.task_not_enabled").to_string())
    }
}

pub(super) fn is_enabled() -> bool {
    let Ok(mut memo) = STATE.lock() else {
        return false;
    };
    memo.get(Instant::now(), || {
        task()
            .and_then(|task| task.State().map_err(|error| error.to_string()))
            .is_ok_and(starts_at_sign_in)
    })
}

pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
    let task = task()?;
    let result = if enabled {
        task.RequestEnableAsync()
            .and_then(|operation| operation.get())
            .map_err(|error| format!("failed to turn on the startup task: {error}"))
            .and_then(enable_outcome)
    } else {
        task.Disable()
            .map_err(|error| format!("failed to turn off the startup task: {error}"))
    };
    if let Ok(mut memo) = STATE.lock() {
        memo.invalidate();
    }
    result
}

pub(super) fn launched_at_sign_in() -> bool {
    AppInstance::GetActivatedEventArgs()
        .and_then(|arguments| arguments.Kind())
        .is_ok_and(|kind| kind == ActivationKind::StartupTask)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/platform/startup/package_task__tests.rs"]
mod tests;
