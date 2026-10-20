use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, TextureHandle};
use rust_i18n::t;

use crate::gui::common::theme;

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct BackgroundPreview {
    cached: Option<(PathBuf, TextureHandle)>,
}

impl BackgroundPreview {
    const BOX_SIZE: egui::Vec2 = egui::Vec2::new(180.0, 120.0);

    const MAX_EDGE: u32 = 512;

    pub fn show(&mut self, ui: &mut egui::Ui, launcher_dir: &Path, path: Option<&Path>) {
        let texture = self.texture(ui.ctx(), launcher_dir, path);
        let (rect, _) = ui.allocate_exact_size(Self::BOX_SIZE, egui::Sense::hover());
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(8),
            theme::ROW_CREAM,
            theme::BORDER,
            egui::StrokeKind::Inside,
        );
        match texture {
            Some((id, size)) => {
                ui.painter().image(
                    id,
                    contain(rect.shrink(4.0), size),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    t!("controller.background.no_image").to_string(),
                    egui::FontId::proportional(theme::font_size::SMALL),
                    theme::MUTED,
                );
            }
        }
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        launcher_dir: &Path,
        path: Option<&Path>,
    ) -> Option<(egui::TextureId, egui::Vec2)> {
        let path = path?;
        let cached = self
            .cached
            .as_ref()
            .is_some_and(|(current, _)| current == path);
        if !cached {
            let image =
                crate::game_launcher_screen::support::assets::load_background(launcher_dir, path)
                    .ok()?;
            let image = if image.width() > Self::MAX_EDGE || image.height() > Self::MAX_EDGE {
                image.resize(
                    Self::MAX_EDGE,
                    Self::MAX_EDGE,
                    ::image::imageops::FilterType::Triangle,
                )
            } else {
                image
            };
            let rgba = image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let texture = ctx.load_texture(
                "controller-background-preview",
                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                egui::TextureOptions::LINEAR,
            );
            self.cached = Some((path.to_path_buf(), texture));
        }
        self.cached
            .as_ref()
            .map(|(_, texture)| (texture.id(), texture.size_vec2()))
    }
}

fn contain(area: egui::Rect, content: egui::Vec2) -> egui::Rect {
    if content.x <= 0.0 || content.y <= 0.0 || area.width() <= 0.0 || area.height() <= 0.0 {
        return area;
    }
    let scale = (area.width() / content.x).min(area.height() / content.y);
    egui::Rect::from_center_size(area.center(), content * scale)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/ui/settings/background_preview__tests.rs"]
mod tests;
