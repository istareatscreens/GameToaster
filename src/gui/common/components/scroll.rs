use eframe::egui;

use crate::gui::common::theme;

pub fn scroll_area<R>(
    ui: &mut egui::Ui,
    area: egui::ScrollArea,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    let body_widgets = ui.visuals().widgets.clone();
    theme::apply_scroll_bar_visuals(ui);
    let output = area.show(ui, |ui| {
        ui.visuals_mut().widgets = body_widgets.clone();
        body(ui)
    });
    ui.visuals_mut().widgets = body_widgets;
    output
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/scroll__tests.rs"]
mod tests;
