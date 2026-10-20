use eframe::egui::{self, Response, RichText};

use crate::gui::common::theme;

pub fn hint(text: impl Into<String>) -> RichText {
    RichText::new(text).small().color(theme::MUTED)
}

pub fn note(text: impl Into<String>) -> RichText {
    RichText::new(text).color(theme::MUTED)
}

pub fn path(text: impl Into<String>) -> RichText {
    RichText::new(text).color(theme::PATH_BLUE)
}

pub fn status(text: impl Into<String>) -> RichText {
    RichText::new(text).color(theme::PATH_BLUE)
}

pub fn warning(text: impl Into<String>) -> RichText {
    RichText::new(text).color(theme::ERROR)
}

pub fn copyable_text(ui: &mut egui::Ui, text: &str, color: Option<egui::Color32>) -> Response {
    let mut view: &str = text;
    let mut edit = egui::TextEdit::multiline(&mut view)
        .desired_width(f32::INFINITY)
        .desired_rows(1)
        .frame(egui::Frame::NONE);
    if let Some(color) = color {
        edit = edit.text_color(color);
    }
    ui.add(edit)
}

pub fn error_text(ui: &mut egui::Ui, text: &str) -> Response {
    copyable_text(ui, text, Some(theme::ERROR))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/text__tests.rs"]
mod tests;
