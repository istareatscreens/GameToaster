use eframe::egui;

pub fn measure(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> egui::Response) -> egui::Vec2 {
    add(&mut ui.new_child(egui::UiBuilder::new().invisible()))
        .rect
        .size()
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/measure__tests.rs"]
mod tests;
