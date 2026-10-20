use std::time::Duration;

use eframe::egui;
use rust_i18n::t;

use crate::game_launcher_screen::{ControllerLaunchSource, ControllerLauncherAction};
use crate::gui::common::theme;
use crate::launcher_gui::dialogs::Dialog;
use crate::launcher_gui::hotkeys::HotkeyAction;
use crate::launcher_gui::tray::TrayAction;
use crate::launcher_gui::views;

use super::launch_monitor::HideTick;
use super::worker::WorkerCommand;
use super::LauncherApp;

impl LauncherApp {
    fn loading_status(&self) -> Option<String> {
        self.busy.clone().or_else(|| {
            self.pending_hide
                .as_ref()
                .map(|pending| t!("status.loading", exe => pending.watch_exe.as_str()).to_string())
        })
    }

    fn controller_launch_status(&self) -> Option<String> {
        if self.controller_launch_pending.is_some() || self.pending_hide.is_some() {
            self.loading_status()
        } else {
            None
        }
    }

    fn controller_credit_readout(&self) -> Option<String> {
        let state = self.game_state.read()?;
        if state.free_play || state.credit_count == 0 {
            return None;
        }
        Some(t!("controller.screen.credits", count => state.credit_count.to_string()).to_string())
    }
}

impl eframe::App for LauncherApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::CANVAS.to_normalized_gamma_f32()
    }

    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(result) = self.worker.receiver.try_recv() {
            self.handle_worker_result(context, result);
        }

        if !self.controller_startup_attempted {
            self.controller_startup_attempted = true;

            let screen = &self.config.settings.controller_screen;
            let inputs = super::startup_screen::StartupScreenInputs {
                enabled: screen.enabled,
                started_in_tray: self.started_in_tray,
                open_on_os_startup: screen.open_on_os_startup,
                launch_on_start: screen.launch_on_start,
                controller_only_start: screen.controller_only_start,
                monitor_return_configured:
                    crate::game_launcher_screen::ControllerLauncher::monitor_return_configured(
                        screen,
                    ),
            };
            let action = super::startup_screen::decide(inputs, || {
                crate::game_launcher_screen::configured_display_available(screen)
            });
            match action {
                super::startup_screen::StartupScreenAction::Nothing => {}
                super::startup_screen::StartupScreenAction::OpenInTray => {
                    self.open_controller();
                }
                super::startup_screen::StartupScreenAction::ArmMonitorReturn
                | super::startup_screen::StartupScreenAction::RevealMonitorMissing => {
                    let armed = action
                        == super::startup_screen::StartupScreenAction::ArmMonitorReturn
                        && self
                            .controller
                            .arm_monitor_return(&self.config.settings.controller_screen);
                    if !armed {
                        self.show_centered(context);
                        self.dialog = Some(Dialog::error(
                            t!("controller.startup.adapter_missing_title").to_string(),
                            t!("controller.startup.adapter_missing_body").to_string(),
                        ));
                    }
                }
                super::startup_screen::StartupScreenAction::OpenForUser { controller_only } => {
                    self.open_controller();
                    if controller_only {
                        if self.controller.is_open() {
                            context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            self.main_visible = false;
                        } else {
                            self.show_centered(context);
                        }
                    }
                }
            }
        }

        for action in self
            .controller
            .poll(&self.config.settings.controller_screen)
        {
            if self.controller.is_suspended()
                && !matches!(
                    action,
                    ControllerLauncherAction::HideLauncherToTray
                        | ControllerLauncherAction::ExitLauncher
                )
            {
                continue;
            }
            self.handle_controller_action(context, action);
        }

        if self.busy.is_none() && !self.closing {
            if let Some(config) = &self.pending_settings_save {
                if self
                    .worker
                    .try_send(WorkerCommand::SaveConfigQuiet {
                        config: config.clone(),
                        retire_backgrounds: self.pending_background_retire.clone(),
                    })
                    .is_ok()
                {
                    self.pending_settings_save = None;
                    self.pending_background_retire.clear();
                }
            }
        }

        if let Some(rx) = &self.store_apps_rx {
            match rx.try_recv() {
                Ok(result) => {
                    if let Some(Dialog::GameSettings(draft)) = &mut self.dialog {
                        draft.store_picker = Some(match result {
                            Ok(apps) => crate::launcher_gui::dialogs::StorePicker::Loaded(apps),
                            Err(error) => crate::launcher_gui::dialogs::StorePicker::Failed(error),
                        });
                    }
                    self.store_apps_rx = None;
                }

                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    context.request_repaint_after(Duration::from_millis(100))
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.store_apps_rx = None,
            }
        }

        if !self.startup_launch_attempted {
            self.startup_launch_attempted = true;
            if self.config.settings.launch_default_on_open {
                match self.config.default_game {
                    Some(id) => self.launch_with_source(id, ControllerLaunchSource::Startup),
                    None => {
                        self.dialog = Some(Dialog::error(
                            t!("errors.default_game_missing"),
                            t!("errors.default_game_missing_detail"),
                        ));
                    }
                }
            }
        }

        if let Some(tray) = &self.tray {
            for action in tray.poll() {
                match action {
                    TrayAction::Restore => {
                        self.show_centered(context);
                    }
                    TrayAction::ToggleMainWindow => {
                        if self.main_visible {
                            context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            self.main_visible = false;
                        } else {
                            self.show_centered(context);
                        }
                    }
                    TrayAction::Launch(id) => {
                        self.launch_with_source(id, ControllerLaunchSource::Tray);
                    }
                    TrayAction::ToggleControllerScreen => {
                        if !self.controller.is_open() {
                            self.open_controller();
                        } else if self.controller.is_suspended() {
                            self.controller.resume();
                        } else {
                            self.controller.suspend();
                        }
                    }
                    TrayAction::Close => {
                        self.begin_shutdown();
                    }
                }
            }
        }

        while let Some(action) = self.hotkeys.try_recv() {
            match action {
                HotkeyAction::OpenLauncher => {
                    self.show_centered(context);
                }
                HotkeyAction::CloseActiveGame => {
                    self.close_active_games();
                }
                HotkeyAction::Launch(id) => {
                    self.launch_with_source(id, ControllerLaunchSource::Hotkey);
                }
                HotkeyAction::Close(id) => {
                    self.close_game(id);
                }
            }
        }

        if let Some(mut pending) = self.pending_hide.take() {
            match pending.tick() {
                HideTick::Wait => {
                    self.pending_hide = Some(pending);
                    context.request_repaint_after(Duration::from_millis(250));
                }
                HideTick::Confirmed => {
                    context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.main_visible = false;
                }
                HideTick::Failed(message) => {
                    self.dialog = Some(Dialog::error(t!("errors.game_did_not_start"), message));
                }
            }
        }

        if super::console_hider::tick(
            &mut self.pending_console_hide,
            self.config.settings.console(),
        ) {
            context.request_repaint_after(Duration::from_millis(250));
        }

        if self.center_frames > 0 {
            if let Some(index) = self.config.settings.adapter {
                self.move_to_adapter(context, index);
            } else if let Some(command) = egui::ViewportCommand::center_on_screen(context) {
                context.send_viewport_cmd(command);
            }
            self.center_frames -= 1;
        }

        let close_requested = context.input(|input| input.viewport().close_requested());
        if close_requested && !self.allow_close {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        if let Some(mut controller_config) = self.controller.render_settings(context) {
            let previous = self
                .config
                .settings
                .controller_screen
                .background_image
                .clone();
            controller_config.background_image =
                crate::game_launcher_screen::resolve_background_image(
                    &self.launcher_dir,
                    controller_config.background_image.take(),
                );
            if let Some(retired) = crate::game_launcher_screen::background_copy_to_retire(
                previous.as_deref(),
                controller_config.background_image.as_deref(),
            ) {
                self.pending_background_retire.push(retired);
            }
            let was_open = self.controller.is_open();
            if was_open {
                self.close_controller();
            }
            self.config.settings.controller_screen = controller_config;
            self.controller.configuration_changed(&self.config);
            self.pending_settings_save = Some(self.config.clone());
            if was_open {
                self.open_controller();
            }
        }
        let controller_status = self.controller_launch_status();
        let controller_credits = self.controller_credit_readout();
        let mut screen_actions = Vec::new();
        let shutdown = self.controller.render(
            context,
            &self.config,
            controller_status.as_deref(),
            controller_credits.as_deref(),
            &mut screen_actions,
        );
        for action in screen_actions {
            self.handle_controller_action(context, action);
        }
        if shutdown {
            self.begin_shutdown();
        }

        let game_running = self.controller.launch_in_progress() && !self.closing;
        let controller_visible = self.controller.is_open() && !self.controller.is_suspended();
        let keep_alive_suspended = self.controller.is_open()
            && self.controller.is_suspended()
            && self
                .config
                .settings
                .controller_screen
                .keep_controller_alive_when_minimized;

        if !game_running {
            if controller_visible {
                if self.controller.is_reconnecting() {
                    context.request_repaint_after(Duration::from_millis(33));
                }
            } else if keep_alive_suspended {
                context.request_repaint_after(Duration::from_millis(100));
            } else if self.center_frames > 0 || self.busy.is_some() || self.closing {
                context.request_repaint_after(Duration::from_millis(80));
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.spacing_mut().item_spacing.y = 0.0;
        let context = ui.ctx().clone();

        let title_action = views::title_bar(ui);
        self.handle_ui_action(&context, title_action);

        let status = self.loading_status();
        let actions = views::main_content(
            ui,
            &self.config,
            &self.plugin_status,
            status.as_deref(),
            self.controller.is_open(),
            &self.controller.status(),
        );
        for action in actions {
            self.handle_ui_action(&context, action);
        }

        ui.painter().rect_stroke(
            context.content_rect(),
            0.0,
            egui::Stroke::new(1.0_f32, theme::BORDER_COLOR),
            egui::StrokeKind::Inside,
        );

        super::pending_notice::show_when_free(&mut self.dialog, &self.config_notice);
        if let Some(dialog) = &mut self.dialog {
            let event = dialog.show(&context);
            self.handle_dialog_event(event);
        }
    }
}
