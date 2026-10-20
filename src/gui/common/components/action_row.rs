use eframe::egui::{self, Response, WidgetText};

use super::{action, measure};

pub fn action_row<R>(
    ui: &mut egui::Ui,
    action_text: impl Into<WidgetText>,
    item: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, Response) {
    let action_text = action_text.into();
    ui.horizontal(|ui| {
        let action_width = measure(ui, |ui| action(ui, action_text.clone())).x;
        let room = (ui.available_width() - ui.spacing().item_spacing.x - action_width).max(0.0);
        let item = ui
            .allocate_ui(egui::vec2(room, ui.spacing().interact_size.y), item)
            .inner;
        (item, action(ui, action_text))
    })
    .inner
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/action_row__tests.rs"]
mod tests;
