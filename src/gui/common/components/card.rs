use eframe::egui::{self, InnerResponse};

use crate::gui::common::theme::frames;

pub fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<R> {
    frames::card().show(ui, add_contents)
}

pub fn compact_card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<R> {
    frames::compact_card().show(ui, add_contents)
}

pub fn section<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<R> {
    frames::section().show(ui, |ui| {
        ui.set_width(ui.available_width());
        add_contents(ui)
    })
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/card__tests.rs"]
mod tests;
