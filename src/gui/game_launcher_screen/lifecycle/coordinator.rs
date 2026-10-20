use crate::launcher_gui::model::GameId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum State {
    Idle,
    QuiescingController,
    Launching,
    Running,
    Restoring,
    ShuttingDown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LaunchSource {
    MainGui,
    Controller,
    Tray,
    Hotkey,
    Startup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum Action {
    HideControllerViewport,
    DisconnectController,
    Spawn(GameId),
    ShowControllerViewport,
    ConnectController,
}

#[derive(Debug)]
pub(in crate::gui::game_launcher_screen) struct LifecycleCoordinator {
    state: State,
    pending: Option<(GameId, LaunchSource)>,
    controller_was_open: bool,
    reopen_after_exit: bool,

    always_reopen: bool,
}

impl LifecycleCoordinator {
    pub fn new(reopen_after_exit: bool) -> Self {
        Self {
            state: State::Idle,
            pending: None,
            controller_was_open: false,
            reopen_after_exit,
            always_reopen: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn set_reopen_after_exit(&mut self, reopen: bool) {
        self.reopen_after_exit = reopen;
    }

    pub fn set_always_reopen(&mut self, always: bool) {
        self.always_reopen = always;
    }

    fn should_reopen(&self) -> bool {
        (self.reopen_after_exit && self.controller_was_open) || self.always_reopen
    }

    pub fn request_launch(
        &mut self,
        game: GameId,
        source: LaunchSource,
        controller_open: bool,
    ) -> Result<Vec<Action>, String> {
        if self.state != State::Idle {
            return Err(format!("launch lifecycle is busy in {:?}", self.state));
        }
        self.pending = Some((game, source));
        self.controller_was_open = controller_open;
        self.state = State::QuiescingController;
        Ok(vec![
            Action::HideControllerViewport,
            Action::DisconnectController,
        ])
    }

    pub fn controller_released(
        &mut self,
        open_handles: usize,
        live_readers: usize,
    ) -> Result<Action, String> {
        if self.state != State::QuiescingController {
            return Err("controller release acknowledgement arrived outside quiescing".into());
        }
        if open_handles != 0 || live_readers != 0 {
            self.state = State::Idle;
            self.pending = None;
            return Err(format!(
                "controller handoff failed closed: {open_handles} handles, {live_readers} readers"
            ));
        }
        let game = self
            .pending
            .ok_or_else(|| "no pending launch".to_string())?
            .0;
        self.state = State::Launching;
        Ok(Action::Spawn(game))
    }

    pub fn launch_succeeded(&mut self) -> Result<(), String> {
        if self.state != State::Launching {
            return Err("launch success arrived outside launching".into());
        }
        self.state = State::Running;
        Ok(())
    }

    pub fn launch_failed(&mut self) -> Vec<Action> {
        if self.state == State::ShuttingDown {
            return Vec::new();
        }
        self.state = State::Idle;
        self.pending = None;
        if self.controller_was_open || self.always_reopen {
            vec![Action::ShowControllerViewport, Action::ConnectController]
        } else {
            Vec::new()
        }
    }

    pub fn game_exited(&mut self) -> Vec<Action> {
        if self.state == State::ShuttingDown {
            return Vec::new();
        }
        if self.state != State::Running {
            return Vec::new();
        }
        self.state = State::Restoring;
        self.pending = None;
        if self.should_reopen() {
            vec![Action::ShowControllerViewport, Action::ConnectController]
        } else {
            self.state = State::Idle;
            Vec::new()
        }
    }

    pub fn restore_complete(&mut self) {
        if self.state == State::Restoring {
            self.state = State::Idle;
        }
    }

    pub fn shutdown(&mut self) {
        self.state = State::ShuttingDown;
        self.pending = None;
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/lifecycle/coordinator__tests.rs"]
mod tests;
