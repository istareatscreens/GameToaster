use eframe::egui::{self, RichText};
use rust_i18n::t;

use crate::gui::common::components::{self, text, ButtonRole, DialogContent};

use super::{plugin_labels, DialogEvent, PluginLibraryDraft};

pub(super) struct PluginLibraryDialog<'a> {
    draft: &'a mut PluginLibraryDraft,
    event: &'a mut DialogEvent,
}

impl<'a> PluginLibraryDialog<'a> {
    pub(super) fn new(draft: &'a mut PluginLibraryDraft, event: &'a mut DialogEvent) -> Self {
        Self { draft, event }
    }
}

impl DialogContent for PluginLibraryDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let event = &mut *self.event;
        ui.horizontal(|ui| {
            if components::action(ui, t!("plugins.import")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(t!("plugins.import_title").to_string())
                    .add_filter(t!("plugins.package_filter").to_string(), &["zip", "toml"])
                    .pick_file()
                {
                    *event = DialogEvent::ImportPlugin(path);
                }
            }
            if components::action(ui, t!("plugins.refresh")).clicked() {
                *event = DialogEvent::RefreshPlugins;
            }
        });
        if let Some(notice) = &draft.notice {
            ui.add_space(6.0);
            ui.label(text::status(notice).small());
        }
        ui.add_space(8.0);

        if draft.plugins.is_empty() {
            ui.label(text::note(t!("plugins.library_empty")));
            return;
        }

        components::Combo::new("plugin-library-revision", &mut draft.selected)
            .choices(
                draft.plugins.iter().enumerate().map(|(index, plugin)| {
                    (Some(index), plugin_labels::plugin_choice_label(plugin))
                }),
            )
            .placeholder(t!("plugins.choose_revision"))
            .full_width()
            .show(ui);

        let Some(plugin) = draft
            .selected
            .and_then(|index| draft.plugins.get(index))
            .cloned()
        else {
            return;
        };

        ui.add_space(8.0);
        components::card(ui, |ui| {
            ui.label(RichText::new(&plugin.manifest.label).strong());
            ui.label(&plugin.manifest.description);
            ui.add_space(4.0);
            ui.monospace(
                t!("plugins.id_line", value => plugin.revision.plugin_id.as_str()).to_string(),
            );
            ui.monospace(
                t!("plugins.version_line", value => plugin.revision.plugin_version.to_string())
                    .to_string(),
            );
            ui.monospace(format!("MD5      {}", plugin.revision.package_md5));
            ui.monospace(format!("SHA-256  {}", plugin.revision.package_sha256));
            ui.label(
                t!(
                    "plugins.validation_line",
                    config => plugin.manifest.config_files.len(),
                    plugin => plugin.manifest.plugin_files.len()
                )
                .to_string(),
            );
            let assignments = draft
                .assigned_games
                .get(&plugin.revision.package_sha256)
                .cloned()
                .unwrap_or_default();
            ui.label(if assignments.is_empty() {
                t!("plugins.assigned_none").to_string()
            } else {
                t!("plugins.assigned", games => assignments.join(", ")).to_string()
            });
        });

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if components::action(ui, t!("plugins.edit_revision")).clicked() {
                *event = DialogEvent::EditPlugin(plugin.clone());
            }
            if components::action(ui, t!("plugins.export_zip")).clicked() {
                let suggested = format!(
                    "{}-{}.zip",
                    plugin.revision.plugin_id, plugin.revision.plugin_version
                );
                if let Some(output) = rfd::FileDialog::new()
                    .set_title(t!("plugins.export_title").to_string())
                    .set_file_name(&suggested)
                    .add_filter(t!("plugins.zip_filter").to_string(), &["zip"])
                    .save_file()
                {
                    *event = DialogEvent::ExportPlugin {
                        plugin: plugin.clone(),
                        output,
                    };
                }
            }
            if components::action(ui, t!("plugins.reveal")).clicked() {
                *event = DialogEvent::RevealPlugin(plugin.revision.root.clone());
            }
        });

        let assigned = draft
            .assigned_games
            .get(&plugin.revision.package_sha256)
            .is_some_and(|games| !games.is_empty());
        ui.add_space(8.0);
        if draft.confirm_remove {
            ui.label(text::warning(t!("plugins.remove_confirm")));
            ui.horizontal(|ui| {
                if components::action(ui, t!("plugins.keep")).clicked() {
                    draft.confirm_remove = false;
                }
                if ui
                    .add_enabled(
                        !assigned,
                        components::button(
                            t!("plugins.move_recovery").to_string(),
                            ButtonRole::Destructive,
                        ),
                    )
                    .clicked()
                {
                    *event = DialogEvent::RemoveCatalogRevision(plugin);
                }
            });
        } else if ui
            .add_enabled(
                !assigned,
                components::button(
                    t!("plugins.remove_revision").to_string(),
                    ButtonRole::Destructive,
                ),
            )
            .on_disabled_hover_text(t!("plugins.unassign_first").to_string())
            .clicked()
        {
            draft.confirm_remove = true;
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("plugins.close")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/plugin_library__tests.rs"]
mod tests;
