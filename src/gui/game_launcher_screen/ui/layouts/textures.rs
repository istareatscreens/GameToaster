use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use eframe::egui::{self, TextureHandle};

use crate::launcher_gui::model::GameId;

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct LayoutTextures {
    textures: BTreeMap<CacheKey, TextureHandle>,
    recent: VecDeque<CacheKey>,

    background: Option<(PathBuf, TextureHandle)>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CacheKey {
    pub(super) game_id: GameId,
    pub(super) target_bucket: u16,
}

const MAX_TEXTURES: usize = 32;

impl LayoutTextures {
    pub fn clear_artwork(&mut self) {
        self.textures.clear();
        self.recent.clear();
    }

    #[cfg(test)]
    pub fn artwork_count(&self) -> usize {
        self.textures.len()
    }

    pub fn texture_for(
        &mut self,
        context: &egui::Context,
        launcher_dir: &Path,
        id: GameId,
        relative: Option<&Path>,
        target_size: f32,
    ) -> Option<&TextureHandle> {
        let relative = relative?;
        let target_bucket = ((target_size / 32.0).round() as u16).max(1);
        let existing = self
            .textures
            .keys()
            .find(|key| key.game_id == id && key.target_bucket == target_bucket)
            .copied();
        if let Some(key) = existing {
            self.touch(key);
            return self.textures.get(&key);
        }
        let (image, content_hash) =
            super::super::super::support::assets::load_rgba_with_hash(launcher_dir, relative)
                .ok()?;
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let key = CacheKey {
            game_id: id,
            target_bucket,
        };
        let texture = context.load_texture(
            format!(
                "controller-game-artwork-{}-{content_hash:016x}-{target_bucket}",
                id.0
            ),
            egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        self.insert_texture(key, texture);
        self.textures.get(&key)
    }

    pub fn background_texture(
        &mut self,
        context: &egui::Context,
        launcher_dir: &Path,
        path: &Path,
    ) -> Option<&TextureHandle> {
        let cached = self
            .background
            .as_ref()
            .is_some_and(|(current, _)| current == path);
        if !cached {
            let image =
                super::super::super::support::assets::load_background(launcher_dir, path).ok()?;

            const MAX_EDGE: u32 = 2048;
            let image = if image.width() > MAX_EDGE || image.height() > MAX_EDGE {
                image.resize(MAX_EDGE, MAX_EDGE, ::image::imageops::FilterType::Triangle)
            } else {
                image
            };
            let rgba = image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let texture = context.load_texture(
                "game-launcher-screen-background",
                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                egui::TextureOptions::LINEAR,
            );
            self.background = Some((path.to_path_buf(), texture));
        }
        self.background.as_ref().map(|(_, texture)| texture)
    }

    pub(super) fn insert_texture(&mut self, key: CacheKey, texture: TextureHandle) {
        while self.textures.len() >= MAX_TEXTURES && !self.textures.contains_key(&key) {
            if let Some(oldest) = self.recent.pop_front() {
                self.textures.remove(&oldest);
            } else {
                break;
            }
        }
        self.textures.insert(key, texture);
        self.touch(key);
    }

    fn touch(&mut self, key: CacheKey) {
        self.recent.retain(|candidate| *candidate != key);
        self.recent.push_back(key);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/textures__tests.rs"]
mod tests;
