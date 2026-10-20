use eframe::egui;
use rust_i18n::t;

use super::super::controller::{self, RELEASE_TIMEOUT};
use super::super::display::geometry::{resolve_profile, PixelRect};
use super::super::surface::circle::circle_rect;
use super::super::ui;
use super::super::ui::settings::{LiveButtons, SettingsAction, SettingsDraft, TestStatus};
use super::super::ControllerScreenConfig;
use super::window::viewport_builder;
use super::ControllerLauncher;

impl ControllerLauncher {
    pub(crate) fn open_settings(
        &mut self,
        config: &ControllerScreenConfig,
        startup_enabled: bool,
    ) -> Result<(), String> {
        self.settings = Some(SettingsDraft::new(config, startup_enabled)?);
        Ok(())
    }

    fn discover_controllers(&self) -> Result<Vec<controller::DiscoveredController>, String> {
        controller::discover()
    }

    pub(crate) fn render_settings(
        &mut self,
        context: &egui::Context,
    ) -> Option<ControllerScreenConfig> {
        let launcher_dir = self.launcher_dir.clone();
        let action = self
            .settings
            .as_mut()
            .map(|draft| ui::settings::show(context, &launcher_dir, draft))
            .unwrap_or(SettingsAction::None);
        let result = match action {
            SettingsAction::None => None,
            SettingsAction::Cancel => {
                if self.test_transient {
                    if let Err(error) = self.manager.disconnect(RELEASE_TIMEOUT) {
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::CouldNotStop(error);
                        }
                        return None;
                    }
                    self.test_transient = false;
                    self.test_pending = false;
                } else if self.test_pending && self.open {
                    if let Some(config) = &self.active_controller_config {
                        let _ = self.manager.connect(config);
                    }
                }
                self.test_pending = false;
                self.settings = None;
                None
            }
            SettingsAction::Save(config) => {
                if self.test_transient {
                    if let Err(error) = self.manager.disconnect(RELEASE_TIMEOUT) {
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::CouldNotStop(error);
                        }
                        return None;
                    }
                    self.test_transient = false;
                    self.test_pending = false;
                }
                self.settings = None;
                Some(config)
            }
            SettingsAction::Test(config) => {
                let transient = !self.open && !self.manager.is_active();
                match self.manager.connect(&config) {
                    Ok(()) => {
                        self.test_pending = true;
                        self.test_transient = transient;
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::Connecting;
                            settings.live_buttons = LiveButtons::Waiting;
                            settings.testing = true;
                            settings.error = None;
                        }
                    }
                    Err(error) => {
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::Failed(error.clone());
                        }
                    }
                }
                None
            }
            SettingsAction::StopTest => {
                if self.test_transient {
                    if let Err(error) = self.manager.disconnect(RELEASE_TIMEOUT) {
                        if let Some(settings) = &mut self.settings {
                            settings.test_status = TestStatus::ReleaseFailed(error.clone());
                        }
                        return None;
                    }
                } else if self.open {
                    if let Some(config) = &self.active_controller_config {
                        if let Err(error) = self.manager.connect(config) {
                            if let Some(settings) = &mut self.settings {
                                settings.test_status = TestStatus::RestoreFailed(error.clone());
                            }
                            return None;
                        }
                    }
                }
                self.test_pending = false;
                self.test_transient = false;
                if let Some(settings) = &mut self.settings {
                    settings.testing = false;
                    settings.test_status = TestStatus::Stopped;
                }
                None
            }
            SettingsAction::AutoDetect => {
                let discovered = self.discover_controllers();
                if let Some(settings) = &mut self.settings {
                    match discovered {
                        Ok(found) => match found.as_slice() {
                            [] => settings.test_status = TestStatus::NoneDetected,
                            [one] => settings.apply_discovery(one),
                            many => settings.test_status = TestStatus::MultipleDetected(many.len()),
                        },
                        Err(error) => {
                            settings.error =
                                Some(t!("controller.error.detect", detail => error).to_string())
                        }
                    }
                }
                None
            }
        };
        self.render_calibration_preview(context);
        result
    }

    fn render_calibration_preview(&mut self, context: &egui::Context) {
        let Some(settings) = self.settings.as_ref().filter(|settings| settings.preview) else {
            return;
        };
        let Some(monitor) = settings.selected().cloned() else {
            return;
        };
        let profile = resolve_profile(settings.value.orientation, monitor.bounds);
        let Ok(circle) = circle_rect(monitor.bounds, settings.diameter_px, profile) else {
            return;
        };
        let builder = viewport_builder(&monitor, "GameToaster Calibration");
        let bounds = monitor.bounds;
        let local_circle = PixelRect {
            x: circle.x - bounds.x,
            y: circle.y - bounds.y,
            width: circle.width,
            height: circle.height,
        };

        let dismiss = context.show_viewport_immediate(
            egui::ViewportId::from_hash_of("gametoaster-controller-calibration"),
            builder,
            |ui, _class| {
                ui::settings::calibration_preview::render(ui, local_circle);
                ui.ctx().input(|input| {
                    input.key_pressed(egui::Key::Escape) || input.viewport().close_requested()
                })
            },
        );
        if dismiss {
            if let Some(settings) = self.settings.as_mut() {
                settings.preview = false;
            }
        }
    }
}

pub(super) fn background_to_draw<'a>(
    settings: Option<&'a SettingsDraft>,
    config: &'a ControllerScreenConfig,
) -> Option<&'a std::path::Path> {
    match settings {
        Some(draft) => draft.value.background_image.as_deref(),
        None => config.background_image.as_deref(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/settings__tests.rs"]
mod tests;
