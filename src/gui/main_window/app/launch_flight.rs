use crate::launcher_gui::model::GameId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Flight {
    pub worker: Option<(GameId, bool)>,
    pub awaiting: Option<GameId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CancelAction {
    CancelWorker(GameId),

    CloseGame(GameId),

    Nothing,
}

impl Flight {
    pub(super) fn cancel_action(self) -> CancelAction {
        match (self.worker, self.awaiting) {
            (Some((_, true)), _) => CancelAction::Nothing,
            (Some((game, false)), _) => CancelAction::CancelWorker(game),
            (None, Some(game)) => CancelAction::CloseGame(game),
            (None, None) => CancelAction::Nothing,
        }
    }

    pub(super) fn cancellable(self) -> bool {
        self.cancel_action() != CancelAction::Nothing
    }

    pub(super) fn close_when_launched(self, game: GameId) -> bool {
        self.worker == Some((game, true))
    }

    pub(super) fn follows_stages(self) -> bool {
        !matches!(self.worker, Some((_, true)))
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/launch_flight__tests.rs"]
mod tests;
