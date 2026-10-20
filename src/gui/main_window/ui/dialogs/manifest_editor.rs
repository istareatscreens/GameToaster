use eframe::egui::{self, RichText};
use rust_i18n::t;

use crate::gui::common::components::{self, text, DialogContent};
use crate::plugin::manifest::{
    ConfigFile, CopyType, InstallScript, Operation, PluginFile, Runtime, WatchType,
};

use super::{DialogEvent, ManifestEditorDraft};

pub(super) struct ManifestEditorDialog<'a> {
    draft: &'a mut ManifestEditorDraft,
    event: &'a mut DialogEvent,
}

impl<'a> ManifestEditorDialog<'a> {
    pub(super) fn new(draft: &'a mut ManifestEditorDraft, event: &'a mut DialogEvent) -> Self {
        Self { draft, event }
    }
}

impl DialogContent for ManifestEditorDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let mut advanced = draft.advanced;
        if components::checkbox(ui, &mut advanced, t!("plugins.editor.advanced").to_string())
            .changed()
        {
            if advanced {
                draft.refresh_raw();
                draft.advanced = true;
            } else {
                match crate::plugin::manifest::Manifest::parse(&draft.raw) {
                    Ok(manifest) => {
                        draft.version_text = manifest.plugin_version.to_string();
                        draft.manifest = manifest;
                        draft.refresh_raw();
                        draft.advanced = false;
                    }
                    Err(error) => {
                        draft.error =
                            Some(t!("plugins.editor.invalid", detail => error).to_string())
                    }
                }
            }
        }

        if draft.advanced {
            ui.label(text::hint(t!("plugins.editor.strict_hint")));
            ui.add(
                egui::TextEdit::multiline(&mut draft.raw)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(28),
            );
        } else {
            let before = (draft.manifest.clone(), draft.version_text.clone());
            structured_editor(ui, draft);
            if (&draft.manifest, &draft.version_text) != (&before.0, &before.1) {
                draft.refresh_raw();
            }
            ui.collapsing(t!("plugins.editor.canonical_preview").to_string(), |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut draft.raw)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(14)
                        .interactive(false),
                );
            });
        }
    }

    fn error(&self) -> Option<&str> {
        self.draft.error.as_deref()
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let event = &mut *self.event;
        if components::action(ui, t!("common.cancel")).clicked() {
            *event = DialogEvent::Cancel;
        }
        if components::action(ui, t!("plugins.editor.validate_import")).clicked() {
            match draft.validated_raw() {
                Ok(manifest) => {
                    *event = DialogEvent::SaveEditedPlugin {
                        plugin: draft.source.clone(),
                        manifest,
                    };
                }
                Err(error) => {
                    draft.error = Some(t!("plugins.editor.invalid", detail => error).to_string())
                }
            }
        }
    }
}

fn structured_editor(ui: &mut egui::Ui, draft: &mut ManifestEditorDraft) {
    ui.label(RichText::new(t!("plugins.editor.identity").to_string()).strong());
    ui.label(
        t!("plugins.editor.manifest_version", version => draft.manifest.manifest_version)
            .to_string(),
    );
    ui.monospace(format!("pluginId = {}", draft.manifest.plugin_id));
    field(ui, &t!("plugins.editor.version"), &mut draft.version_text);
    field(ui, &t!("plugins.editor.label"), &mut draft.manifest.label);
    ui.label(t!("plugins.editor.description").to_string());
    ui.add(
        egui::TextEdit::multiline(&mut draft.manifest.description)
            .desired_width(f32::INFINITY)
            .desired_rows(3),
    );

    ui.add_space(8.0);
    ui.label(RichText::new(t!("plugins.editor.launch_watch").to_string()).strong());
    optional_field(ui, "gameLauncher", &mut draft.manifest.game_launcher);
    optional_field(ui, "executableWatch", &mut draft.manifest.executable_watch);
    components::Combo::new(
        "manifest-watch-type",
        &mut draft.manifest.executable_watch_type,
    )
    .choice(None, t!("plugins.editor.not_specified"))
    .choice(Some(WatchType::GameDirectory), "game_directory")
    .choice(Some(WatchType::FindInDirectory), "find_in_directory")
    .show(ui);

    ui.add_space(8.0);
    components::checkbox(
        ui,
        &mut draft.manifest.is_patcher,
        t!("plugins.editor.patcher_runtime").to_string(),
    );
    if draft.manifest.is_patcher {
        let runtime = draft.manifest.runtime.get_or_insert_with(|| Runtime {
            executable: String::new(),
            operations: vec![Operation::Prepare, Operation::Restore],
        });
        field(
            ui,
            &t!("plugins.editor.runtime_executable"),
            &mut runtime.executable,
        );
        ui.horizontal_wrapped(|ui| {
            operation_toggle(ui, &mut runtime.operations, Operation::Prepare, "prepare");
            operation_toggle(ui, &mut runtime.operations, Operation::Launch, "launch");
            operation_toggle(ui, &mut runtime.operations, Operation::Restore, "restore");
        });
    } else {
        draft.manifest.runtime = None;
    }

    ui.add_space(10.0);
    config_rows(ui, draft);
    ui.add_space(10.0);
    plugin_rows(ui, draft);
    ui.add_space(10.0);
    install_script_rows(ui, draft);
}

fn config_rows(ui: &mut egui::Ui, draft: &mut ManifestEditorDraft) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("plugins.editor.config_files").to_string()).strong());
        if components::action(ui, t!("plugins.editor.add_row").to_string()).clicked() {
            draft.manifest.config_files.push(ConfigFile {
                config_file_location: String::new(),
                config_file_copy_type: CopyType::GameFolder,
                config_file_replace_path: None,
                config_add_to_config_menu: false,
            });
        }
    });
    let mut remove = None;
    for (index, row) in draft.manifest.config_files.iter_mut().enumerate() {
        components::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if components::action(ui, t!("common.remove").to_string()).clicked() {
                    remove = Some(index);
                }
            });
            field(
                ui,
                &t!("plugins.editor.source"),
                &mut row.config_file_location,
            );
            copy_type(ui, ("config-copy", index), &mut row.config_file_copy_type);
            optional_field(
                ui,
                &t!("plugins.editor.replace_path"),
                &mut row.config_file_replace_path,
            );
            components::checkbox(
                ui,
                &mut row.config_add_to_config_menu,
                t!("plugins.editor.add_config_menu").to_string(),
            );
        });
        ui.add_space(4.0);
    }
    if let Some(index) = remove {
        draft.manifest.config_files.remove(index);
    }
}

fn plugin_rows(ui: &mut egui::Ui, draft: &mut ManifestEditorDraft) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("plugins.editor.plugin_files").to_string()).strong());
        if components::action(ui, t!("plugins.editor.add_row").to_string()).clicked() {
            draft.manifest.plugin_files.push(PluginFile {
                plugin_file_location: String::new(),
                plugin_file_copy_type: CopyType::GameFolder,
                plugin_file_replace_path: None,
            });
        }
    });
    let mut remove = None;
    for (index, row) in draft.manifest.plugin_files.iter_mut().enumerate() {
        components::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if components::action(ui, t!("common.remove").to_string()).clicked() {
                    remove = Some(index);
                }
            });
            field(
                ui,
                &t!("plugins.editor.source"),
                &mut row.plugin_file_location,
            );
            copy_type(ui, ("plugin-copy", index), &mut row.plugin_file_copy_type);
            optional_field(
                ui,
                &t!("plugins.editor.replace_path"),
                &mut row.plugin_file_replace_path,
            );
        });
        ui.add_space(4.0);
    }
    if let Some(index) = remove {
        draft.manifest.plugin_files.remove(index);
    }
}

fn install_script_rows(ui: &mut egui::Ui, draft: &mut ManifestEditorDraft) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(t!("plugins.editor.install_scripts").to_string()).strong());
        if components::action(ui, t!("plugins.editor.add_row").to_string()).clicked() {
            draft.manifest.install_scripts.push(InstallScript {
                install_script_location: String::new(),
                install_script_arguments: Vec::new(),
            });
        }
    });
    let mut remove_script = None;
    for (script_index, script) in draft.manifest.install_scripts.iter_mut().enumerate() {
        components::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", script_index + 1));
                if components::action(ui, t!("common.remove").to_string()).clicked() {
                    remove_script = Some(script_index);
                }
            });
            field(
                ui,
                &t!("plugins.editor.source"),
                &mut script.install_script_location,
            );
            ui.label(t!("plugins.editor.install_script_arguments").to_string());
            let mut remove_argument = None;
            for (argument_index, argument) in script.install_script_arguments.iter_mut().enumerate()
            {
                let (_, remove) = components::action_row(ui, t!("common.remove"), |ui| {
                    ui.add(egui::TextEdit::singleline(argument).desired_width(f32::INFINITY))
                });
                if remove.clicked() {
                    remove_argument = Some(argument_index);
                }
            }
            if let Some(argument_index) = remove_argument {
                script.install_script_arguments.remove(argument_index);
            }
            if components::action(ui, t!("plugins.editor.add_argument").to_string()).clicked() {
                script.install_script_arguments.push(String::new());
            }
        });
        ui.add_space(4.0);
    }
    if let Some(script_index) = remove_script {
        draft.manifest.install_scripts.remove(script_index);
    }
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
}

fn optional_field(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) {
    let mut enabled = value.is_some();
    ui.horizontal(|ui| {
        components::checkbox(ui, &mut enabled, label);
        if enabled {
            ui.add(
                egui::TextEdit::singleline(value.get_or_insert_with(String::new))
                    .desired_width(f32::INFINITY),
            );
        }
    });
    if !enabled {
        *value = None;
    }
}

fn copy_type(ui: &mut egui::Ui, id: impl std::hash::Hash, value: &mut CopyType) {
    components::Combo::new(id, value)
        .choices(
            [
                CopyType::GameFolder,
                CopyType::FindAndReplace,
                CopyType::FindAndReplaceInDirectory,
                CopyType::Absolute,
            ]
            .map(|option| (option, option.to_string())),
        )
        .show(ui);
}

fn operation_toggle(
    ui: &mut egui::Ui,
    operations: &mut Vec<Operation>,
    operation: Operation,
    label: &str,
) {
    let mut enabled = operations.contains(&operation);
    if components::checkbox(ui, &mut enabled, label).changed() {
        operations.retain(|candidate| *candidate != operation);
        if enabled {
            operations.push(operation);
            operations.sort();
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/manifest_editor__tests.rs"]
mod tests;
