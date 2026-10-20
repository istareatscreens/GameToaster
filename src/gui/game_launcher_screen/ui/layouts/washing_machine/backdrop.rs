use std::path::Path;

use eframe::egui::{self, Color32, Stroke, Vec2};

use super::WashingMachineLayout;
use crate::gui::common::theme;

fn cover_uv(area: Vec2, tex: Vec2) -> egui::Rect {
    let full = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if area.x <= 0.0 || area.y <= 0.0 || tex.x <= 0.0 || tex.y <= 0.0 {
        return full;
    }
    let area_aspect = area.x / area.y;
    let tex_aspect = tex.x / tex.y;
    if tex_aspect > area_aspect {
        let u = area_aspect / tex_aspect;
        let x0 = (1.0 - u) / 2.0;
        egui::Rect::from_min_max(egui::pos2(x0, 0.0), egui::pos2(x0 + u, 1.0))
    } else {
        let v = tex_aspect / area_aspect;
        let y0 = (1.0 - v) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(1.0, y0 + v))
    }
}

fn fill_circle_with_texture(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    texture_id: egui::TextureId,
    tex_size: Vec2,
) {
    let uv = cover_uv(Vec2::splat(radius * 2.0), tex_size);

    let to_uv = |p: egui::Pos2| {
        let fx = (p.x - (center.x - radius)) / (radius * 2.0);
        let fy = (p.y - (center.y - radius)) / (radius * 2.0);
        egui::pos2(uv.left() + fx * uv.width(), uv.top() + fy * uv.height())
    };
    let mut mesh = egui::Mesh::with_texture(texture_id);
    mesh.vertices.push(egui::epaint::Vertex {
        pos: center,
        uv: to_uv(center),
        color: Color32::WHITE,
    });
    let segments = 96u32;
    for i in 0..=segments {
        let angle = std::f32::consts::TAU * (i as f32 / segments as f32);
        let pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: to_uv(pos),
            color: Color32::WHITE,
        });
    }
    for i in 1..=segments {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

impl WashingMachineLayout {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_backdrop(
        &mut self,
        ui: &egui::Ui,
        launcher_dir: &Path,
        painter: &egui::Painter,
        background: Option<&Path>,
        center: egui::Pos2,
        radius: f32,
        scale: f32,
    ) {
        painter.rect_filled(ui.max_rect(), 0.0, Color32::BLACK);
        let background = background.and_then(|path| {
            self.cache
                .background_texture(ui.ctx(), launcher_dir, path)
                .map(|texture| (texture.id(), texture.size_vec2()))
        });
        if let Some((texture_id, tex_size)) = background {
            fill_circle_with_texture(painter, center, radius, texture_id, tex_size);
        } else {
            painter.circle_filled(center, radius, theme::LIST_PANEL);
        }
        painter.circle_stroke(
            center,
            radius - (5.0 * scale).max(1.0),
            Stroke::new((10.0 * scale).max(2.0), theme::SKY_TOP),
        );
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/washing_machine/backdrop__tests.rs"]
mod tests;
