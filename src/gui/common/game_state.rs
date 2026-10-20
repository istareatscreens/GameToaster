use std::sync::{Arc, RwLock};

use crate::plugin::runtime::{GameMode, GameState};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameLiveState {
    pub free_play: bool,
    pub credit_count: u32,
    pub active_paid_play: bool,
    pub mode: Option<GameMode>,
}

pub fn should_block_game_exit(setting_enabled: bool, state: Option<GameLiveState>) -> bool {
    if !setting_enabled {
        return false;
    }
    let Some(state) = state else {
        return false;
    };
    if state.free_play {
        return false;
    }
    let paid_mode = matches!(state.mode, Some(GameMode::Ready | GameMode::ActivePlay));
    state.credit_count > 0 || state.active_paid_play || paid_mode
}

pub trait GameStateSource {
    fn read(&self) -> Option<GameLiveState>;
}

#[derive(Clone, Default)]
pub struct GameStateHub {
    state: Arc<RwLock<Option<GameLiveState>>>,
}

impl GameStateHub {
    pub fn apply(&self, state: &GameState) {
        let live = state.payment_known().then(|| GameLiveState {
            free_play: state.free_play,
            credit_count: state.credits.unwrap_or(0),
            active_paid_play: state.paid_play.unwrap_or(false),
            mode: (state.mode != GameMode::Unknown).then_some(state.mode),
        });
        if let Ok(mut current) = self.state.write() {
            *current = live;
        }
    }

    pub fn clear(&self) {
        if let Ok(mut current) = self.state.write() {
            *current = None;
        }
    }
}

impl GameStateSource for GameStateHub {
    fn read(&self) -> Option<GameLiveState> {
        self.state.read().ok().and_then(|state| *state)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/game_state__tests.rs"]
mod tests;
