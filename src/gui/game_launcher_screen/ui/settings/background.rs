use eframe::egui;
use rust_i18n::t;

use super::SettingsDraft;

pub(super) fn show(ui: &mut egui::Ui, launcher_dir: &std::path::Path, draft: &mut SettingsDraft) {
    ui.heading(t!("controller.background.heading").to_string());
    ui.label(t!("controller.background.hint").to_string());
    ui.horizontal(|ui| {
        if crate::gui::common::components::action(
            ui,
            t!("controller.background.choose").to_string(),
        )
        .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(t!("controller.file_dialog.background_title").to_string())
                .add_filter(
                    t!("controller.file_dialog.image_filter").as_ref(),
                    &["png", "jpg", "jpeg"],
                )
                .pick_file()
            {
                draft.value.background_image = Some(path);
            }
        }
        ui.add_enabled_ui(draft.value.background_image.is_some(), |ui| {
            if crate::gui::common::components::action(
                ui,
                t!("controller.background.clear").to_string(),
            )
            .clicked()
            {
                draft.value.background_image = None;
            }
        });
        let current = draft
            .value
            .background_image
            .as_ref()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| t!("controller.background.none").to_string());

        ui.add(egui::Label::new(current).truncate());
    });

    ui.add_space(6.0);
    draft
        .background_preview
        .show(ui, launcher_dir, draft.value.background_image.as_deref());
}
