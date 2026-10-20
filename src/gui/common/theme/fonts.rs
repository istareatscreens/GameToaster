use std::collections::BTreeMap;

use eframe::egui::{self, FontFamily, FontId, TextStyle};

use super::font_size;

pub fn install_fonts(context: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let cjk: [(&str, &str); 3] = [
        ("cjk_sc", r"C:\Windows\Fonts\msyh.ttc"),
        ("cjk_tc", r"C:\Windows\Fonts\msjh.ttc"),
        ("cjk_jp", r"C:\Windows\Fonts\meiryo.ttc"),
    ];
    for (name, path) in cjk {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
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
