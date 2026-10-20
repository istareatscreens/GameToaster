use eframe::egui;
use rust_i18n::t;

use crate::game_launcher_screen::{ControllerLaunchSource, ControllerLauncherAction};
use crate::launcher_gui::dialogs::Dialog;
use crate::launcher_gui::model::{GameId, LauncherConfig};
use crate::support::process::handle::CreationTime;

use super::worker::WorkerCommand;
use super::{console_hider, LauncherApp};

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
        let controller_was_open = match self.controller.prepare_game_handoff(
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

        if let Some(watch) = game.watch_target() {
            console_hider::follow_watch_target(
                &mut self.pending_console_hide,
                self.config.settings.console(),
                &watch,
            );
        }
        self.controller_launch_pending = Some((id, controller_was_open));
        if !self.submit(
            WorkerCommand::Launch {
                game,
                config: self.config.clone(),
            },
            t!("busy.launching"),
        ) {
            self.controller_launch_pending = None;
            if self.controller.launch_failed() {
                self.open_controller();
            }
        }
    }

    pub(super) fn hide_consoles_of(&mut self, pid: u32, created: CreationTime) {
        console_hider::follow_root(
            &mut self.pending_console_hide,
            self.config.settings.console(),
            pid,
            created,
        );
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
        self.submit(WorkerCommand::CloseGame { game }, t!("busy.closing"));
    }

    pub(super) fn game_ended(&mut self, game_id: GameId, exit_observed: bool) {
        self.plugin_status.remove(&game_id);
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
            let (px, py) = monitor
                .centered_origin((1080.0 * ppp).round() as i32, (800.0 * ppp).round() as i32);
            context.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                px as f32 / ppp,
                py as f32 / ppp,
            )));
        }
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
