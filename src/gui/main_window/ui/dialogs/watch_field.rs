use eframe::egui;
use rust_i18n::t;

use crate::gui::common::components;

pub(super) fn show(ui: &mut egui::Ui, watch_exe: &mut String, placeholder: &str) {
    let (_, browse) = components::action_row(ui, t!("common.browse"), |ui| {
        ui.add(
            egui::TextEdit::singleline(watch_exe)
                .desired_width(f32::INFINITY)
                .hint_text(placeholder.to_string()),
        )
    });
    if browse.clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(t!("picker.watch_exe").to_string())
            .add_filter(t!("picker.filter_exe"), &["exe"])
            .pick_file()
        {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                *watch_exe = name.to_string();
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/watch_field__tests.rs"]
mod tests;
