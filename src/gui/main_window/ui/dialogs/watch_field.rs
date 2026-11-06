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
        if let Some(path) = crate::gui::common::file_dialog::file_dialog()
            .set_title(t!("picker.watch_exe").to_string())
            .add_filter(t!("picker.filter_exe"), &["exe"])
            .pick_file()
        {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                *watch_exe = name.to_string();
            }
        }
    }
    if names_script(watch_exe) {
        components::text::error_text(ui, &t!("gs.watch_script_invalid"));
    }
}

pub(super) fn names_script(text: &str) -> bool {
    crate::support::process::image::is_script(std::path::Path::new(text.trim()))
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/watch_field__tests.rs"]
mod tests;
