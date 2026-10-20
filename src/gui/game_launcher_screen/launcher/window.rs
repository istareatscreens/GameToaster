use std::time::{Duration, Instant};

use eframe::egui;

use super::super::display::geometry::PixelRect;
use super::super::display::monitor::MonitorDescriptor;
use super::super::display::monitor_watch::MonitorTarget;
use super::super::display::placement;
use super::super::domain::command;
use super::super::ui::layouts::{LauncherFrame, LauncherFrameMode};
use super::super::ControllerLauncherAction;
use super::monitor_return::MonitorReturnState;
use super::resolved_display::resolve_display;
use super::status::ControllerStatus;
use super::ControllerLauncher;
use crate::launcher_gui::model::LauncherConfig;

const VIEWPORT_ID: &str = "gametoaster-game-launcher-screen";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PlacementAttempt {
    attempts: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameVisibility {
    Hidden,

    Menu,

    Launching,
}

fn frame_visibility(
    open: bool,
    suspended: bool,
    launching: bool,
    preview: bool,
) -> FrameVisibility {
    if preview {
        FrameVisibility::Hidden
    } else if open && !suspended {
        FrameVisibility::Menu
    } else if launching {
        FrameVisibility::Launching
    } else {
        FrameVisibility::Hidden
    }
}

impl ControllerLauncher {
    pub(crate) fn suspend(&mut self) {
        if self.open {
            self.suspended = true;

            self.placement = None;
        }
    }

    pub(crate) fn resume(&mut self) {
        self.suspended = false;
        self.force_display_refresh = true;
    }

    pub(crate) fn is_suspended(&self) -> bool {
        self.suspended
    }

    pub(crate) fn render(
        &mut self,
        context: &egui::Context,
        config: &LauncherConfig,
        loading_status: Option<&str>,
        credit_readout: Option<&str>,
        actions: &mut Vec<ControllerLauncherAction>,
    ) -> bool {
        let preview = self
            .settings
            .as_ref()
            .is_some_and(|settings| settings.preview);
        if !self.viewport_spawned && (preview || !self.open) {
            return false;
        }

        let mut visibility =
            frame_visibility(self.open, self.suspended, loading_status.is_some(), preview);

        if visibility != FrameVisibility::Hidden
            && (self.force_display_refresh
                || self.last_display_refresh.elapsed() >= Duration::from_secs(1))
        {
            if self.force_display_refresh {
                self.placement = Some(PlacementAttempt::default());
            }
            self.force_display_refresh = false;
            self.last_display_refresh = Instant::now();
            let shape = self
                .layouts
                .surface_shape(&config.settings.controller_screen.launcher_layout);
            match resolve_display(&config.settings.controller_screen, shape) {
                Ok(display) => self.display = Some(display),
                Err(error) => {
                    self.diagnostics.error("display-topology", &error.logged);
                    let monitor_target =
                        MonitorTarget::from_config(&config.settings.controller_screen);
                    let _ = self.close();
                    self.monitor_return = monitor_target
                        .map(MonitorReturnState::Waiting)
                        .unwrap_or_default();
                    self.status = ControllerStatus::DisplayChanged(error.shown);
                    visibility = FrameVisibility::Hidden;
                }
            }
        }
        let Some(display) = self.display.clone() else {
            return false;
        };
        let bounds = display.monitor.bounds;

        let local_surface = PixelRect {
            x: display.surface.x - bounds.x,
            y: display.surface.y - bounds.y,
            width: display.surface.width,
            height: display.surface.height,
        };
        let layout_id = &config.settings.controller_screen.launcher_layout;
        let mut layout = if visibility != FrameVisibility::Hidden {
            match self.layouts.active_mut(layout_id) {
                Ok(layout) => Some(layout),
                Err(error) => {
                    self.status = ControllerStatus::Error(error.shown);
                    visibility = FrameVisibility::Hidden;
                    None
                }
            }
        } else {
            None
        };
        let visible = visibility != FrameVisibility::Hidden;
        let launching_visible = visibility == FrameVisibility::Launching;
        let builder = viewport_builder(&display.monitor, "GameToaster Controller Launcher")
            .with_visible(visible);
        let viewport_id = egui::ViewportId::from_hash_of(VIEWPORT_ID);

        let connecting = self.reconnect.is_some();

        let background = super::settings::background_to_draw(
            self.settings.as_ref(),
            &config.settings.controller_screen,
        )
        .map(std::path::Path::to_path_buf);
        let (edge, close_requested) =
            context.show_viewport_immediate(viewport_id, builder, |ui, _class| {
                if !visible {
                    return (None, false);
                }
                let Some(layout) = layout.as_deref_mut() else {
                    return (None, false);
                };
                if launching_visible {
                    ui.ctx().request_repaint_after(Duration::from_millis(100));
                    let frame = LauncherFrame {
                        launcher_dir: &self.launcher_dir,
                        config,
                        background: background.as_deref(),
                        games: &self.games,
                        surface_px: local_surface,
                        mode: LauncherFrameMode::Launching {
                            status: loading_status.unwrap_or_default(),
                        },
                    };
                    let _ = layout.render(ui, &frame);
                    let close = ui.ctx().input(|input| input.viewport().close_requested());
                    return (None, close);
                }
                if connecting {
                    ui.ctx().request_repaint_after(Duration::from_millis(33));
                }
                let frame = LauncherFrame {
                    launcher_dir: &self.launcher_dir,
                    config,
                    background: background.as_deref(),
                    games: &self.games,
                    surface_px: local_surface,
                    mode: LauncherFrameMode::Menu { credit_readout },
                };
                let edge = layout.render(ui, &frame);
                let close = ui.ctx().input(|input| input.viewport().close_requested());
                (edge, close)
            });

        self.viewport_spawned = true;
        if visible {
            self.drive_placement(context, viewport_id, bounds);
        } else {
            self.placement = None;
        }

        if let Some(command) = edge {
            command::apply(
                command,
                &config.settings.controller_screen,
                &mut self.games,
                actions,
            );
        }
        if close_requested {
            let _ = self.close();
        }
        false
    }

    fn drive_placement(
        &mut self,
        context: &egui::Context,
        viewport_id: egui::ViewportId,
        bounds: PixelRect,
    ) {
        let Some(mut attempt) = self.placement else {
            return;
        };
        let (pixels_per_point, reported) = context.input_for(viewport_id, |input| {
            (input.pixels_per_point, input.viewport().outer_rect)
        });
        let target = placement::target_in_points(bounds, pixels_per_point);
        match placement::decide(target, reported, attempt.attempts) {
            placement::Placement::Converged => {
                self.placement = None;
                self.diagnostics.transition(
                    "display-placement",
                    format!("converged;attempts={}", attempt.attempts),
                );
            }
            placement::Placement::GiveUp => {
                self.placement = None;
                self.diagnostics.error(
                    "display-placement",
                    &format!(
                        "gave up after {} attempts; target={:?}+{:?} reported={reported:?}",
                        attempt.attempts, target.position, target.size
                    ),
                );
            }
            placement::Placement::Issue => {
                context.send_viewport_cmd_to(
                    viewport_id,
                    egui::ViewportCommand::OuterPosition(target.position),
                );
                context.send_viewport_cmd_to(
                    viewport_id,
                    egui::ViewportCommand::InnerSize(target.size),
                );
                attempt.attempts += 1;
                self.placement = Some(attempt);

                context.request_repaint_after(Duration::from_millis(50));
            }
        }
    }
}

pub(super) fn viewport_builder(monitor: &MonitorDescriptor, title: &str) -> egui::ViewportBuilder {
    let scale = (monitor.dpi_x as f32 / 96.0).max(0.5);
    let bounds = monitor.bounds;
    let size = egui::vec2(bounds.width as f32 / scale, bounds.height as f32 / scale);
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_position(egui::pos2(bounds.x as f32 / scale, bounds.y as f32 / scale))
        .with_inner_size(size)
        .with_min_inner_size(size)
        .with_max_inner_size(size)
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false)
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/window__tests.rs"]
mod tests;
