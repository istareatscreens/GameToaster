use eframe::egui;
use rust_i18n::t;

use crate::game_launcher_screen::{ControllerLaunchSource, ControllerLauncherAction};
use crate::launcher_gui::dialogs::Dialog;
use crate::launcher_gui::model::{GameId, LauncherConfig};
use crate::launcher_gui::runtime::{HandoffGate, LaunchCancel};
use crate::support::process::handle::CreationTime;

use super::launch_flight::{CancelAction, Flight};
use super::launch_monitor::{reports_gone, Presence};
use super::placement::{self, MAIN_WINDOW_POINTS};
use super::worker::WorkerCommand;
use super::LauncherApp;

impl LauncherApp {
    pub(super) fn submit(&mut self, command: WorkerCommand, label: impl Into<String>) -> bool {
        if self.busy.is_some() || self.closing {
            return false;
        }
        match self.worker.try_send(command) {
            Ok(()) => {
                self.busy = Some(label.into());
                true
            }
            Err(error) => {
                self.dialog = Some(Dialog::error(t!("errors.busy"), error));
                false
            }
        }
    }

    pub(super) fn is_plugin_game(&self, game_id: GameId) -> bool {
        self.config
            .game(game_id)
            .is_some_and(|game| game.plugin.is_some())
    }

    pub(super) fn program_presence(
        &mut self,
        context: &egui::Context,
        game_id: GameId,
        presence: Presence,
    ) {
        match presence {
            Presence::Gone => {
                let exe = self
                    .config
                    .game(game_id)
                    .and_then(|game| game.watch_target())
                    .unwrap_or_default();
                self.gone_game = Some(game_id);
                self.dialog = Some(Dialog::error(
                    t!("errors.game_no_longer_running"),
                    t!("errors.game_no_longer_running_detail", exe => exe),
                ));
                if !self.main_visible {
                    self.show_centered(context);
                }
            }
            Presence::Back => self.forget_gone(game_id),
        }
    }

    pub(super) fn forget_gone(&mut self, game_id: GameId) {
        if self.gone_game == Some(game_id) {
            self.gone_game = None;
            if reports_gone(self.dialog.as_ref()) {
                self.dialog = None;
            }
        }
    }

    pub(super) fn launch(&mut self, id: GameId) {
        self.launch_with_source(id, ControllerLaunchSource::MainGui);
    }

    pub(super) fn launch_with_source(&mut self, id: GameId, source: ControllerLaunchSource) {
        if self.busy.is_some() || self.closing {
            return;
        }

        if self.controller.launch_in_progress() {
            return;
        }
        let Some(game) = self.config.game(id).cloned() else {
            self.dialog = Some(Dialog::error(
                t!("errors.game_not_found"),
                t!("errors.game_not_found_detail", id => id),
            ));
            return;
        };
        let controller_was_open = match self.controller.begin_game_launch(
            id,
            source,
            self.config
                .settings
                .controller_screen
                .reopen_after_game_close,
            self.config.settings.controller_screen.starts_main_hidden(),
        ) {
            Ok(was_open) => was_open,
            Err(error) => {
                self.dialog = Some(Dialog::error(
                    rust_i18n::t!("controller.dialog.release_failed_title").to_string(),
                    rust_i18n::t!("controller.dialog.release_failed_body", detail => error)
                        .to_string(),
                ));
                return;
            }
        };

        if let (Some(console_watch), Some(watch)) = (&self.console_watch, game.watch_target()) {
            console_watch.follow_watch_target(id, &watch, game.watch_target_path());
        }
        self.controller_launch_pending = Some((id, controller_was_open));
        let cancel = LaunchCancel::new();
        self.launch_cancel = Some((id, cancel.clone()));
        let handoff = HandoffGate::new();
        self.launch_handoff = Some((id, handoff.clone()));
        if !self.submit(
            WorkerCommand::Launch {
                game,
                config: self.config.clone(),
                cancel,
                handoff,
            },
            t!("busy.launching"),
        ) {
            self.controller_launch_pending = None;
            self.launch_cancel = None;
            self.launch_handoff = None;
            if self.controller.launch_failed() {
                self.open_controller();
            }
        }
    }

    pub(super) fn launch_flight(&self) -> Flight {
        Flight {
            worker: self
                .launch_cancel
                .as_ref()
                .map(|(game, cancel)| (*game, cancel.is_cancelled())),
            awaiting: self
                .pending_hide
                .as_ref()
                .map(|pending| pending.game_id)
                .or(self.gone_game),
        }
    }

    pub(super) fn cancel_launch(&mut self) {
        match self.launch_flight().cancel_action() {
            CancelAction::CancelWorker(game) => {
                if let Some((_, cancel)) = self
                    .launch_cancel
                    .as_ref()
                    .filter(|(launching, _)| *launching == game)
                {
                    cancel.cancel();
                }
                self.busy = Some(t!("busy.cancelling_launch").to_string());
            }
            CancelAction::CloseGame(game) => {
                if self
                    .pending_hide
                    .as_ref()
                    .is_some_and(|pending| pending.reported_by(self.dialog.as_ref()))
                    || reports_gone(self.dialog.as_ref())
                {
                    self.dialog = None;
                }
                self.close_game(game);
            }
            CancelAction::Nothing => {}
        }
    }

    pub(super) fn follow_program(&mut self, game: Option<GameId>, pid: u32, created: CreationTime) {
        if let Some(console_watch) = &self.console_watch {
            match game {
                Some(game) => console_watch.follow_game(game, pid, created),
                None => console_watch.follow_transient(pid, created),
            }
        }
    }

    pub(super) fn end_console_session(&mut self, game: GameId) {
        if let Some(console_watch) = &self.console_watch {
            console_watch.end_game(game);
        }
    }

    pub(super) fn close_game(&mut self, id: GameId) {
        if crate::gui::common::game_state::should_block_game_exit(
            self.config.settings.block_exit_on_credit,
            self.game_state.read(),
        ) {
            self.dialog = Some(Dialog::error(
                rust_i18n::t!("dialog.exit_blocked_title").to_string(),
                rust_i18n::t!("dialog.exit_blocked_body").to_string(),
            ));
            return;
        }

        let Some(game) = self.config.game(id).cloned() else {
            self.dialog = Some(Dialog::error(
                t!("errors.game_not_found"),
                t!("errors.game_not_found_detail", id => id),
            ));
            return;
        };

        if self.pending_hide.as_ref().map(|pending| pending.game_id) == Some(id) {
            self.pending_hide = None;
        }
        self.forget_gone(id);
        self.submit(WorkerCommand::CloseGame { game }, t!("busy.closing"));
    }

    pub(super) fn game_ended(&mut self, game_id: GameId, exit_observed: bool) {
        self.plugin_status.remove(&game_id);
        self.forget_gone(game_id);
        self.end_console_session(game_id);

        if self.pending_hide.as_ref().map(|pending| pending.game_id) == Some(game_id) {
            self.pending_hide = None;
        }
        if self.controller_running_game == Some(game_id) {
            self.controller_running_game = None;
            let should_restore = self.controller.game_exited();
            if should_restore && !self.closing {
                self.open_controller();
            }
            self.controller.restore_complete();
        }
        if exit_observed {
            self.controller
                .turn_lights_off_after_game(&self.config.settings.controller_screen);
        }
    }

    pub(super) fn close_active_games(&mut self) {
        if self.launch_flight().cancellable() {
            self.cancel_launch();
            return;
        }
        if let Some(id) = self.controller_running_game {
            self.close_game(id);
            return;
        }
        let games = self.config.games.clone();
        self.submit(
            WorkerCommand::CloseActiveGames { games },
            t!("busy.closing"),
        );
    }

    pub(super) fn commit_config(&mut self, config: LauncherConfig) {
        self.submit(
            WorkerCommand::SaveConfig { config },
            t!("busy.saving_settings"),
        );
    }

    pub(super) fn show_centered(&mut self, context: &egui::Context) {
        context.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        context.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        context.send_viewport_cmd(egui::ViewportCommand::Focus);

        self.center_frames = self.center_frames.max(4);
        self.main_visible = true;
    }

    pub(super) fn open_controller(&mut self) {
        if !self.config.settings.controller_screen.enabled {
            return;
        }
        if let Err(error) = self
            .controller
            .open(&self.config.settings.controller_screen)
        {
            self.log
                .event("controller-open-failed", &[("error", error.logged)]);
            self.dialog = Some(Dialog::error(
                rust_i18n::t!("controller.dialog.screen_open_failed_title").to_string(),
                error.shown,
            ));
        }
    }

    pub(super) fn close_controller(&mut self) {
        if let Err(error) = self.controller.close() {
            self.dialog = Some(Dialog::error(
                rust_i18n::t!("controller.dialog.disconnect_failed_title").to_string(),
                error,
            ));
        }
    }

    pub(super) fn handle_controller_action(
        &mut self,
        context: &egui::Context,
        action: ControllerLauncherAction,
    ) {
        match action {
            ControllerLauncherAction::Launch(id) => {
                self.launch_with_source(id, ControllerLaunchSource::Controller)
            }
            ControllerLauncherAction::ToggleMainWindow => {
                if self.main_visible {
                    context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.main_visible = false;
                } else {
                    self.show_centered(context);
                }
            }
            ControllerLauncherAction::HideLauncherToTray => {
                if self.controller.is_suspended() {
                    self.controller.resume();
                    if self.minimized_restore_main {
                        self.show_centered(context);
                    }
                    self.minimized_restore_main = false;
                } else if self
                    .config
                    .settings
                    .controller_screen
                    .keep_controller_alive_when_minimized
                    && self.controller.is_open()
                {
                    self.minimized_restore_main = self.main_visible;
                    self.controller.suspend();
                    if self.main_visible {
                        context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                        self.main_visible = false;
                    }
                } else {
                    self.hide_all_to_tray(context);
                }
            }
            ControllerLauncherAction::CancelLaunch => self.cancel_launch(),
            ControllerLauncherAction::ExitLauncher => {
                self.begin_shutdown();
            }
        }
    }

    pub(super) fn hide_all_to_tray(&mut self, context: &egui::Context) {
        self.close_controller();
        context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.main_visible = false;
    }

    pub(super) fn move_to_adapter(&self, context: &egui::Context, index: u32) {
        if let Some(monitor) = crate::launcher_gui::monitors::enumerate()
            .iter()
            .find(|monitor| monitor.index == index)
        {
            let ppp = context.pixels_per_point().max(0.5);
            let (width, height) = main_window_pixels(ppp);
            let (px, py) = monitor.centered_origin(width, height);
            context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                px as f32 / ppp,
                py as f32 / ppp,
            )));
        }
    }

    pub(super) fn circle_position(&self, context: &egui::Context) -> Option<egui::Pos2> {
        let circle = crate::game_launcher_screen::main_window_circle(
            &self.config.settings.controller_screen,
        )?;
        let monitors = crate::launcher_gui::monitors::enumerate();
        let monitor = match self.config.settings.adapter {
            Some(index) => monitors.iter().find(|monitor| monitor.index == index),
            None => monitors.iter().find(|monitor| monitor.primary),
        }?;
        let ppp = context.pixels_per_point().max(0.5);
        let (x, y) = placement::on_circle(main_window_pixels(ppp), monitor.index, Some(circle))?;
        Some(egui::pos2(x as f32 / ppp, y as f32 / ppp))
    }

    pub(super) fn begin_shutdown(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        self.controller_launch_pending = None;
        self.controller_running_game = None;
        self.controller.shutdown_lifecycle();
        if let Err(error) = self.controller.close() {
            self.dialog = Some(Dialog::error(
                rust_i18n::t!("controller.dialog.disconnect_failed_title").to_string(),
                error,
            ));
            self.closing = false;
            return;
        }
        self.busy = Some(t!("busy.shutting_down").to_string());
        if let Err(error) = self.worker.shutdown() {
            self.dialog = Some(Dialog::error(t!("errors.shutdown_failed"), error));
            self.closing = false;
        }
    }
}

fn main_window_pixels(ppp: f32) -> (i32, i32) {
    (
        (MAIN_WINDOW_POINTS.0 * ppp).round() as i32,
        (MAIN_WINDOW_POINTS.1 * ppp).round() as i32,
    )
}
