mod affordances;
mod backdrop;
mod metrics;
mod pills;

use eframe::egui::{self, Color32, FontId, Stroke, Vec2};
use egui_phosphor::regular as icons;
use rust_i18n::t;

use super::super::super::domain::command::LauncherCommand;
use super::super::super::surface::circle::metrics::{safe_chord_width, ARTWORK_UNITS};
use super::super::super::surface::SurfaceShape;
use super::super::canvas::circle_layout;
use super::textures::LayoutTextures;
use super::{LauncherFrame, LauncherFrameMode, LauncherLayoutDescriptor, LauncherScreenLayout};
use crate::gui::common::theme;
use crate::launcher_gui::model::LauncherConfig;
use metrics::{
    ROW_CONTENT_INSET, ROW_HALF_HEIGHT, ROW_HEIGHT, ROW_PITCH, SELECTED_MAX_WIDTH,
    SELECTED_ROW_RING_INSET, UNSELECTED_MAX_WIDTH, VISIBLE_ROWS,
};
use pills::{paint_credit_readout, paint_loading_status};

static WASHING_MACHINE: LauncherLayoutDescriptor = LauncherLayoutDescriptor {
    id: "washing-machine",
    label_key: "controller.layout.washing_machine",
    surface: SurfaceShape::Circle,
};

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct WashingMachineLayout {
    cache: LayoutTextures,
}

impl LauncherScreenLayout for WashingMachineLayout {
    fn descriptor(&self) -> &'static LauncherLayoutDescriptor {
        &WASHING_MACHINE
    }

    fn library_changed(&mut self, config: &LauncherConfig) {
        WashingMachineLayout::library_changed(self, config);
    }

    fn deactivate(&mut self) {
        WashingMachineLayout::deactivate(self);
    }

    fn render(&mut self, ui: &mut egui::Ui, frame: &LauncherFrame<'_>) -> Option<LauncherCommand> {
        match frame.mode {
            LauncherFrameMode::Menu { credit_readout } => {
                WashingMachineLayout::render(self, ui, frame, credit_readout)
            }
            LauncherFrameMode::Launching { status } => {
                WashingMachineLayout::render_launching(self, ui, frame, status);
                None
            }
        }
    }

    #[cfg(test)]
    fn resource_count(&self) -> usize {
        WashingMachineLayout::resource_count(self)
    }
}

fn row_label_left(has_artwork: bool, content_left: f32, artwork_size: f32, scale: f32) -> f32 {
    if has_artwork {
        content_left + artwork_size + 24.0 * scale
    } else {
        content_left
    }
}

impl WashingMachineLayout {
    pub fn library_changed(&mut self, _config: &LauncherConfig) {
        self.cache.clear_artwork();
    }

    pub fn deactivate(&mut self) {
        self.cache.clear_artwork();
    }

    #[cfg(test)]
    pub fn resource_count(&self) -> usize {
        self.cache.artwork_count()
    }

    pub fn render_launching(&mut self, ui: &mut egui::Ui, frame: &LauncherFrame<'_>, status: &str) {
        let (circle, scale) = circle_layout(ui, frame.surface_px);
        let center = circle.center();
        let radius = circle.width() / 2.0;
        let painter = ui.painter().clone();
        self.paint_backdrop(
            ui,
            frame.launcher_dir,
            &painter,
            frame.background,
            center,
            radius,
            scale,
        );
        paint_loading_status(&painter, center, radius, scale, status);
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        frame: &LauncherFrame<'_>,
        credit_readout: Option<&str>,
    ) -> Option<LauncherCommand> {
        let (launcher_dir, config, games) = (frame.launcher_dir, frame.config, frame.games);
        let (circle, scale) = circle_layout(ui, frame.surface_px);
        let center = circle.center();
        let radius = circle.width() / 2.0;
        let painter = ui.painter().clone();

        self.paint_backdrop(
            ui,
            launcher_dir,
            &painter,
            frame.background,
            center,
            radius,
            scale,
        );

        let visible = games.visible_window(VISIBLE_ROWS);
        if visible.is_empty() {
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                t!("controller.screen.empty"),
                FontId::proportional((26.0 * scale).max(13.0)),
                theme::INK,
            );
        } else {
            let row_height = ROW_PITCH * scale;
            let center_index = visible
                .iter()
                .position(|id| Some(*id) == games.selected())
                .unwrap_or(visible.len() / 2);
            for (index, id) in visible.into_iter().enumerate() {
                let Some(game) = config.game(id) else {
                    continue;
                };
                let y = center.y + (index as f32 - center_index as f32) * row_height;
                let relative_y = y - center.y;
                let selected = Some(id) == games.selected();

                let safe = safe_chord_width(
                    circle.width(),
                    relative_y,
                    ROW_HALF_HEIGHT * scale,
                    60.0 * scale,
                )
                .max(1.0);
                let row_width = if selected {
                    safe.min(SELECTED_MAX_WIDTH * scale)
                } else {
                    safe.min(UNSELECTED_MAX_WIDTH * scale)
                };
                let half_height = ROW_HEIGHT * scale / 2.0;
                let top = y - half_height;
                let bottom = y + half_height;
                let left = center.x - row_width / 2.0;

                let selected_reach = (radius - (SELECTED_ROW_RING_INSET * scale).max(1.0)).max(0.0);
                let right = if selected {
                    center.x + selected_reach
                } else {
                    center.x + row_width / 2.0
                };
                let row =
                    egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
                let row_painter = painter.with_clip_rect(row);
                if selected {
                    let corner = (20.0 * scale).clamp(2.0, 40.0).min(half_height);
                    let corner_samples = 6;
                    let arc_samples = 40;
                    let mut points = Vec::with_capacity(corner_samples * 2 + arc_samples + 4);
                    let tl = egui::pos2(left + corner, top + corner);
                    for i in 0..=corner_samples {
                        let a =
                            std::f32::consts::PI * (1.0 + 0.5 * i as f32 / corner_samples as f32);
                        points.push(tl + Vec2::new(a.cos(), a.sin()) * corner);
                    }
                    for i in 0..=arc_samples {
                        let arc_y = top + (bottom - top) * (i as f32 / arc_samples as f32);
                        let dy = arc_y - center.y;
                        let arc_x =
                            center.x + (selected_reach * selected_reach - dy * dy).max(0.0).sqrt();
                        points.push(egui::pos2(arc_x.max(left + corner), arc_y));
                    }
                    let bl = egui::pos2(left + corner, bottom - corner);
                    for i in 0..=corner_samples {
                        let a =
                            std::f32::consts::PI * (0.5 + 0.5 * i as f32 / corner_samples as f32);
                        points.push(bl + Vec2::new(a.cos(), a.sin()) * corner);
                    }
                    painter.add(egui::Shape::convex_polygon(
                        points,
                        theme::PRIMARY_ORANGE,
                        Stroke::NONE,
                    ));

                    let arc_x_at = |arc_y: f32| {
                        let dy = arc_y - center.y;
                        (center.x + (selected_reach * selected_reach - dy * dy).max(0.0).sqrt())
                            .max(left + corner)
                    };
                    let mut border = Vec::with_capacity(corner_samples * 2 + 4);
                    border.push(egui::pos2(arc_x_at(bottom), bottom));
                    for i in 0..=corner_samples {
                        let a =
                            std::f32::consts::PI * (0.5 + 0.5 * i as f32 / corner_samples as f32);
                        border.push(bl + Vec2::new(a.cos(), a.sin()) * corner);
                    }
                    for i in 0..=corner_samples {
                        let a =
                            std::f32::consts::PI * (1.0 + 0.5 * i as f32 / corner_samples as f32);
                        border.push(tl + Vec2::new(a.cos(), a.sin()) * corner);
                    }
                    border.push(egui::pos2(arc_x_at(top), top));
                    painter.add(egui::Shape::line(border, theme::BORDER));
                } else {
                    row_painter.rect(
                        row,
                        egui::CornerRadius::same((20.0 * scale).clamp(2.0, 40.0) as u8),
                        theme::ROW_CREAM,
                        theme::BORDER,
                        egui::StrokeKind::Inside,
                    );
                }

                let content_left = row.left() + ROW_CONTENT_INSET * scale;
                let artwork_size = ARTWORK_UNITS * scale;

                if let Some(relative) = game.controller_image.as_deref() {
                    let artwork_rect = egui::Rect::from_center_size(
                        egui::pos2(content_left + artwork_size / 2.0, y),
                        Vec2::splat(artwork_size),
                    );
                    if let Some(texture) = self.cache.texture_for(
                        ui.ctx(),
                        launcher_dir,
                        game.id,
                        Some(relative),
                        artwork_size,
                    ) {
                        row_painter.image(
                            texture.id(),
                            artwork_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        row_painter.rect(
                            artwork_rect,
                            egui::CornerRadius::same((10.0 * scale).clamp(2.0, 20.0) as u8),
                            theme::ICON_TILE,
                            theme::BORDER,
                            egui::StrokeKind::Inside,
                        );
                        row_painter.text(
                            artwork_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            icons::IMAGE,
                            FontId::proportional(artwork_size * 0.5),
                            theme::MUTED,
                        );
                    }
                }
                let label_left = row_label_left(
                    game.controller_image.is_some(),
                    content_left,
                    artwork_size,
                    scale,
                );
                row_painter.text(
                    egui::pos2(label_left, y),
                    egui::Align2::LEFT_CENTER,
                    &game.label,
                    FontId::proportional((30.0 * scale).max(14.0)),
                    theme::INK,
                );
            }
        }

        if let Some(text) = credit_readout {
            paint_credit_readout(&painter, center, radius, scale, text);
        }

        self.render_edge_controls(ui, &painter, config, center, radius, scale)
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/washing_machine/mod__tests.rs"]
mod tests;
