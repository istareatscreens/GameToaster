use std::fmt;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const ANSWER_TIMEOUT: Duration = Duration::from_secs(30);

const SLICE: Duration = Duration::from_millis(100);

#[derive(Clone, Default)]
pub struct HandoffGate {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    state: Mutex<State>,
    answered: Condvar,
}

#[derive(Default, Clone, PartialEq, Eq)]
enum State {
    #[default]
    Idle,
    Asked,
    Answered(Result<(), String>),
}

impl HandoffGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request(&self, cancelled: &dyn Fn() -> bool) -> Result<(), String> {
        {
            let mut state = self.lock();
            if let State::Answered(answer) = &*state {
                return answer.clone();
            }
            *state = State::Asked;
        }
        self.inner.answered.notify_all();
        let mut waited = Duration::ZERO;
        let mut state = self.lock();
        while waited < ANSWER_TIMEOUT {
            if let State::Answered(answer) = &*state {
                return answer.clone();
            }
            if cancelled() {
                return Err("the launch was cancelled while the controller was released".into());
            }
            let (next, _) = self
                .inner
                .answered
                .wait_timeout(state, SLICE)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = next;
            waited += SLICE;
        }
        Err("the launcher did not release the controller in time".into())
    }

    pub fn answer(&self, outcome: Result<(), String>) {
        {
            let mut state = self.lock();
            if matches!(*state, State::Answered(_)) {
                return;
            }
            *state = State::Answered(outcome);
        }
        self.inner.answered.notify_all();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl fmt::Debug for HandoffGate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match &*self.lock() {
            State::Idle => "idle",
            State::Asked => "asked",
            State::Answered(Ok(())) => "released",
            State::Answered(Err(_)) => "refused",
        };
        formatter
            .debug_struct("HandoffGate")
            .field("state", &state)
            .finish()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/runtime/handoff__tests.rs"]
mod tests;
