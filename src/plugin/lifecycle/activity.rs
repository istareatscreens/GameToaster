use std::sync::{Arc, Mutex};

use super::watch::WatchControl;

#[derive(Default)]
struct ActivityState {
    plugin_work: usize,
    managed_games: usize,
}

struct ActivityInner {
    state: Mutex<ActivityState>,
    watch: Arc<WatchControl>,
}

#[derive(Clone)]
pub struct ActivityGuard {
    inner: Arc<ActivityInner>,
}

impl ActivityGuard {
    pub(super) fn new(watch: Arc<WatchControl>) -> Self {
        Self {
            inner: Arc::new(ActivityInner {
                state: Mutex::new(ActivityState::default()),
                watch,
            }),
        }
    }

    pub fn try_plugin_operation(&self) -> Result<PluginWorkLease, String> {
        {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "managed-game activity lock is poisoned".to_string())?;
            if state.managed_games != 0 {
                return Err(format!(
                    "plugin work is suspended while {} managed game process(es) are active",
                    state.managed_games
                ));
            }
            if state.plugin_work != 0 {
                return Err("another plugin operation is already active".into());
            }
            state.plugin_work = 1;
        }
        if let Err(error) = self.inner.watch.pause() {
            if let Ok(mut state) = self.inner.state.lock() {
                state.plugin_work = 0;
            }
            return Err(error);
        }
        Ok(PluginWorkLease {
            inner: Arc::clone(&self.inner),
            active: true,
        })
    }

    #[cfg(test)]
    pub fn active_game_count(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| state.managed_games)
            .unwrap_or(usize::MAX)
    }

    pub fn reserve_game_activity(&self) -> Result<ManagedGameActivity, String> {
        self.try_plugin_operation()?.reserve_game()
    }
}

pub struct PluginWorkLease {
    inner: Arc<ActivityInner>,
    active: bool,
}

impl PluginWorkLease {
    pub(super) fn reserve_game(mut self) -> Result<ManagedGameActivity, String> {
        {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| "managed-game activity lock is poisoned".to_string())?;
            if state.plugin_work != 1 || state.managed_games != 0 {
                return Err("invalid plugin-work to managed-game handoff".into());
            }
            state.plugin_work = 0;
            state.managed_games = 1;
        }
        self.active = false;
        Ok(ManagedGameActivity {
            inner: Arc::clone(&self.inner),
            active: true,
        })
    }
}

impl Drop for PluginWorkLease {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Ok(mut state) = self.inner.state.lock() {
            state.plugin_work = state.plugin_work.saturating_sub(1);
            if state.plugin_work == 0 && state.managed_games == 0 {
                drop(state);
                self.inner.watch.resume();
            }
        }
    }
}

pub struct ManagedGameActivity {
    inner: Arc<ActivityInner>,
    active: bool,
}

impl ManagedGameActivity {
    #[cfg(test)]
    pub fn active_game_count(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| state.managed_games)
            .unwrap_or(usize::MAX)
    }
}

impl Drop for ManagedGameActivity {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Ok(mut state) = self.inner.state.lock() {
            state.managed_games = state.managed_games.saturating_sub(1);
            if state.managed_games == 0 && state.plugin_work == 0 {
                drop(state);
                self.inner.watch.resume();
            }
        }
        self.active = false;
    }
}
