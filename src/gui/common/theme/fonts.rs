use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui::{self, FontFamily, FontId, TextStyle};

use super::font_size;
use crate::support::system_folders;

const CJK_CANDIDATES: [(&str, &[&str]); 3] = [
    ("cjk_sc", &["msyh.ttc"]),
    ("cjk_tc", &["msjh.ttc"]),
    ("cjk_jp", &["meiryo.ttc", "YuGothM.ttc"]),
];

pub(crate) fn select_cjk_fonts(
    folders: &[PathBuf],
    present: impl Fn(&Path) -> bool,
) -> Vec<(&'static str, PathBuf)> {
    CJK_CANDIDATES
        .iter()
        .filter_map(|(family, candidates)| {
            candidates
                .iter()
                .find_map(|name| {
                    folders
                        .iter()
                        .map(|folder| folder.join(name))
                        .find(|path| present(path))
                })
                .map(|path| (*family, path))
        })
        .collect()
}

pub fn install_fonts(context: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let folders = system_folders::fonts_folders();
    for (name, path) in select_cjk_fonts(&folders, |path| path.is_file()) {
        if let Ok(bytes) = std::fs::read(&path) {
            fonts
                .font_data
                .insert(name.to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push(name.to_owned());
            }
        }
    }
    context.set_fonts(fonts);
}

pub(super) fn apply_text_styles(styles: &mut BTreeMap<TextStyle, FontId>) {
    styles.insert(
        TextStyle::Heading,
        FontId::new(font_size::HEADING, FontFamily::Proportional),
    );
    styles.insert(
        TextStyle::Body,
        FontId::new(font_size::BODY, FontFamily::Proportional),
    );
    styles.insert(
        TextStyle::Button,
        FontId::new(font_size::BODY, FontFamily::Proportional),
    );
    styles.insert(
        TextStyle::Monospace,
        FontId::new(font_size::COMPACT, FontFamily::Monospace),
    );
    styles.insert(
        TextStyle::Small,
        FontId::new(font_size::SMALL, FontFamily::Proportional),
    );
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/theme/fonts__tests.rs"]
mod tests;
