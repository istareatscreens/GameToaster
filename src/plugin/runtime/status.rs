use super::channel::{GameState, Message, Progress};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeStatus {
    pub progress: Option<Progress>,
    pub game_state: Option<GameState>,
}

impl RuntimeStatus {
    pub fn apply(&mut self, message: &Message) {
        match message {
            Message::Progress(progress) => self.progress = Some(progress.clone()),
            Message::GameState(state) => self.game_state = Some(*state),
            Message::Result(_) | Message::Log(_) => {}
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/runtime/status__tests.rs"]
mod tests;
