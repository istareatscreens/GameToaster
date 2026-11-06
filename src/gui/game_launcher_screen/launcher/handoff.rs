use rust_i18n::t;

use super::super::controller::RELEASE_TIMEOUT;
use super::super::lifecycle;
use super::super::lifecycle::coordinator::Action as LifecycleAction;
use super::super::ControllerLaunchSource;
use super::status::ControllerStatus;
use super::ControllerLauncher;
use std::time::Instant;

use crate::launcher_gui::model::GameId;

impl ControllerLauncher {
    pub(crate) fn launch_in_progress(&self) -> bool {
        self.lifecycle.state() != lifecycle::coordinator::State::Idle
    }

    pub(crate) fn begin_game_launch(
        &mut self,
        game: GameId,
        source: ControllerLaunchSource,
        reopen_after_exit: bool,
        always_reopen: bool,
    ) -> Result<bool, String> {
        self.cancel_lights_off();
        let was_open = self.open;
        self.lifecycle.set_reopen_after_exit(reopen_after_exit);

        self.lifecycle.set_always_reopen(always_reopen);
        self.lifecycle.request_launch(game, source, was_open)?;
        self.diagnostics.transition(
            "launch-begin",
            format!("game={game};source={source:?};viewport_open={was_open}"),
        );
        self.open = false;
        self.suspended = false;
        self.reconnect = None;

        self.cancel_arming.begin();
        self.launch_started = Some(Instant::now());

        self.placement = None;

        Ok(was_open)
    }

    pub(crate) fn complete_game_handoff(&mut self, game: GameId) -> Result<(), String> {
        match self.lifecycle.begin_handoff() {
            Ok(_) => {}
            Err(error) => return Err(error),
        }
        self.diagnostics
            .transition("handoff-begin", format!("game={game}"));
        self.layouts.release_resources();
        match self.manager.disconnect(RELEASE_TIMEOUT) {
            Ok(proof) if proof.open_handles == 0 && proof.live_readers == 0 => {
                match self
                    .lifecycle
                    .controller_released(proof.open_handles, proof.live_readers)?
                {
                    LifecycleAction::Spawn(pending) if pending == game => {}
                    _ => return Err(t!("controller.dialog.invalid_spawn").to_string()),
                }
                self.status = ControllerStatus::Released;
                self.diagnostics
                    .transition("handoff-complete", "handles=0;readers=0");
                Ok(())
            }
            Ok(proof) => {
                self.lifecycle.launch_failed();
                Err(t!(
                    "controller.status.handoff_failed",
                    handles => proof.open_handles,
                    readers => proof.live_readers
                )
                .to_string())
            }
            Err(error) => {
                self.lifecycle.launch_failed();
                Err(error)
            }
        }
    }

    pub(crate) fn launch_succeeded(&mut self) -> Result<(), String> {
        self.lifecycle.launch_succeeded()
    }

    pub(crate) fn launch_failed(&mut self) -> bool {
        self.lifecycle
            .launch_failed()
            .contains(&LifecycleAction::ShowControllerViewport)
    }

    pub(crate) fn game_exited(&mut self) -> bool {
        self.lifecycle
            .game_exited()
            .contains(&LifecycleAction::ShowControllerViewport)
    }

    pub(crate) fn restore_complete(&mut self) {
        self.lifecycle.restore_complete();
    }

    pub(crate) fn shutdown_lifecycle(&mut self) {
        self.lifecycle.shutdown();
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/handoff__tests.rs"]
mod tests;
