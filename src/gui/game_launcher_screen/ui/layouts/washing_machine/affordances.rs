use eframe::egui::{self, FontId, Sense, Stroke, Vec2};
use egui_phosphor::regular as icons;

use super::super::super::super::domain::command::LauncherCommand;
use super::WashingMachineLayout;
use crate::gui::common::theme;
use crate::launcher_gui::model::LauncherConfig;

pub(super) fn edge_controls() -> [(f32, &'static str, LauncherCommand); 5] {
    [
        (-157.5, icons::ARROW_UP, LauncherCommand::Previous),
        (-67.5, icons::X, LauncherCommand::LauncherX),
        (-22.5, icons::GEAR, LauncherCommand::ToggleMainWindow),
        (67.5, icons::CHECK, LauncherCommand::Launch),
        (157.5, icons::ARROW_DOWN, LauncherCommand::Next),
    ]
}

pub(super) fn teardrop(
    center: egui::Pos2,
    angle_deg: f32,
    radius: f32,
    scale: f32,
) -> (Vec<egui::Pos2>, egui::Pos2, f32) {
    let angle = angle_deg.to_radians();
    let radial = Vec2::new(angle.cos(), angle.sin());
    let lobe_r = (44.0 * scale).max(9.0);

    let tip_reach = lobe_r * 1.85;
    let tip_radius = radius - (5.0 * scale).max(1.0);
    let tip = center + radial * tip_radius;
    let lobe_center = center + radial * (tip_radius - tip_reach);

    let alpha = (lobe_r / tip_reach).clamp(-1.0, 1.0).acos();
    const ARC_SAMPLES: usize = 24;
    let mut outline = Vec::with_capacity(ARC_SAMPLES + 2);
    outline.push(tip);

    for i in 0..=ARC_SAMPLES {
        let theta = (angle + alpha)
            + (std::f32::consts::TAU - 2.0 * alpha) * (i as f32 / ARC_SAMPLES as f32);
        outline.push(lobe_center + Vec2::new(theta.cos(), theta.sin()) * lobe_r);
    }
    (outline, lobe_center, lobe_r)
}

impl WashingMachineLayout {
    pub(super) fn render_edge_controls(
        &self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        config: &LauncherConfig,
        center: egui::Pos2,
        radius: f32,
        scale: f32,
    ) -> Option<LauncherCommand> {
        let screen = &config.settings.controller_screen;
        let mut clicked = None;
        for (angle_deg, icon, action) in edge_controls() {
            if !action.enabled(screen) {
                continue;
            }
            let (outline, lobe_center, lobe_r) = teardrop(center, angle_deg, radius, scale);
            let hit = egui::Rect::from_center_size(lobe_center, Vec2::splat(lobe_r * 2.0));
            let response = ui.interact(
                hit,
                ui.id().with(("cls-edge", action as u8)),
                Sense::click(),
            );
            let fill = if response.hovered() {
                theme::PRIMARY_ORANGE
            } else {
                theme::SKY_BOTTOM
            };
            painter.add(egui::Shape::convex_polygon(
                outline,
                fill,
                Stroke::new((2.0 * scale).max(1.0), theme::BORDER_COLOR),
            ));
            painter.text(
                lobe_center,
                egui::Align2::CENTER_CENTER,
                icon,
                FontId::proportional(lobe_r * 1.05),
                theme::PATH_BLUE,
            );
            if response.clicked() {
                clicked = Some(action);
            }
        }
        clicked
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/washing_machine/affordances__tests.rs"]
mod tests;
