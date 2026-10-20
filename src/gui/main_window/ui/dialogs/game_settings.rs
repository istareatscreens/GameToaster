use std::path::PathBuf;

use eframe::egui::{self, RichText, Vec2};
use egui_phosphor::regular as icons;
use rust_i18n::t;

use crate::gui::common::components::{self, text, ButtonRole, DialogContent};
use crate::gui::common::theme;
use crate::launcher_gui::model::CloseMode;
use crate::plugin::paths::ManagedFileKind;

use super::drafts::{GameSettingsDraft, PluginInspectionState, StorePicker};
use super::{plugin_labels, DialogEvent};

const CHOICE_SIZE: egui::Vec2 = egui::Vec2::new(82.0, 26.0);

pub(super) struct GameSettingsDialog<'a> {
    draft: &'a mut GameSettingsDraft,
    event: &'a mut DialogEvent,
}

impl<'a> GameSettingsDialog<'a> {
    pub(super) fn new(draft: &'a mut GameSettingsDraft, event: &'a mut DialogEvent) -> Self {
        Self { draft, event }
    }
}

impl DialogContent for GameSettingsDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let event = &mut *self.event;
        ui.label(RichText::new(&draft.label).strong());
        ui.add_space(8.0);

        show_plugin_settings(ui, draft, event);
        ui.add_space(12.0);

        ui.label(RichText::new(t!("gs.watch_title").to_string()).strong());
        ui.label(text::hint(t!("gs.watch_hint")));
        super::watch_field::show(ui, &mut draft.watch_exe, &t!("gs.watch_placeholder"));

        ui.add_space(12.0);
        ui.label(RichText::new(t!("gs.close_action").to_string()).strong());
        ui.horizontal(|ui| {
            if ui
                .add(components::choice_button(
                    t!("gs.kill_game"),
                    draft.close_mode == CloseMode::KillExe,
                    CHOICE_SIZE,
                ))
                .clicked()
            {
                draft.close_mode = CloseMode::KillExe;
            }
            if ui
                .add(components::choice_button(
                    t!("gs.run_script"),
                    draft.close_mode == CloseMode::Script,
                    CHOICE_SIZE,
                ))
                .clicked()
            {
                draft.close_mode = CloseMode::Script;
            }
        });
        if draft.close_mode == CloseMode::Script {
            ui.add_space(6.0);
            let shown = draft
                .close_script
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| t!("gs.no_script").to_string());

            let (_, choose) = components::action_row(ui, t!("gs.choose_bat"), |ui| {
                ui.add(egui::Label::new(text::path(shown)).truncate())
            });
            if choose.clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(t!("picker.close_script").to_string())
                    .add_filter(t!("picker.filter_script"), &["bat", "cmd"])
                    .pick_file()
                {
                    draft.close_script = Some(path);
                }
            }
        }

        ui.add_space(12.0);
        ui.label(RichText::new(t!("gs.config_menu").to_string()).strong());
        ui.label(text::hint(t!("gs.config_hint")));
        ui.add_space(4.0);
        let mut remove_index = None;
        if !draft.config_targets.is_empty() {
            components::scroll_area(
                ui,
                egui::ScrollArea::vertical()
                    .id_salt("config-targets")
                    .max_height(124.0)
                    .auto_shrink([false, true]),
                |ui| {
                    for (index, target) in draft.config_targets.iter().enumerate() {
                        components::compact_card(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_height(24.0);
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add(
                                                components::borderless_button(
                                                    t!("common.remove").to_string(),
                                                    ButtonRole::Destructive,
                                                )
                                                .min_size(Vec2::new(64.0, 24.0)),
                                            )
                                            .clicked()
                                        {
                                            remove_index = Some(index);
                                        }
                                        ui.add_space(8.0);
                                        let name = target
                                            .file_name()
                                            .and_then(|name| name.to_str())
                                            .unwrap_or("?");
                                        ui.add(egui::Label::new(text::path(name)).truncate())
                                            .on_hover_text(target.display().to_string());
                                    },
                                );
                            });
                        });
                        ui.add_space(4.0);
                    }
                },
            );
        }
        if let Some(index) = remove_index {
            draft.config_targets.remove(index);
        }
        ui.horizontal(|ui| {
            if components::action(ui, t!("gs.add_target")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(t!("picker.add_target").to_string())
                    .add_filter(
                        t!("picker.filter_config_or_program"),
                        &["ini", "txt", "cfg", "json", "exe", "bat", "cmd"],
                    )
                    .add_filter(t!("picker.filter_all"), &["*"])
                    .pick_file()
                {
                    draft.config_targets.push(path);
                }
            }

            if components::action(ui, t!("gs.add_store")).clicked() {
                draft.store_picker = Some(StorePicker::Loading);
                *event = DialogEvent::RequestStoreApps;
            }
        });

        if draft.store_picker.is_some() {
            let mut chosen: Option<String> = None;
            let mut close = false;
            components::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t!("gs.pick_store").to_string()).strong());
                    let pad = (ui.available_width() - 28.0).max(8.0);
                    ui.add_space(pad);

                    if ui
                        .add(
                            components::button(
                                RichText::new(icons::X)
                                    .size(theme::font_size::COMPACT)
                                    .color(theme::INK),
                                ButtonRole::Destructive,
                            )
                            .min_size(Vec2::new(28.0, 24.0)),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
                ui.add_space(4.0);

                let has_apps = matches!(&draft.store_picker, Some(StorePicker::Loaded(apps)) if !apps.is_empty());
                if has_apps {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.store_filter)
                            .desired_width(f32::INFINITY)
                            .hint_text(t!("gs.search_store").to_string()),
                    );
                    ui.add_space(4.0);
                }
                let needle = draft.store_filter.trim().to_lowercase();
                match &draft.store_picker {
                    Some(StorePicker::Loading) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(text::note(t!("gs.loading_store")));
                        });
                    }
                    Some(StorePicker::Failed(error)) => {
                        components::error_text(ui, &t!("gs.store_failed", detail => error));
                    }
                    Some(StorePicker::Loaded(apps)) if apps.is_empty() => {
                        ui.label(text::note(t!("gs.no_store")));
                    }
                    Some(StorePicker::Loaded(apps)) => {
                        components::scroll_area(
                            ui,
                            egui::ScrollArea::vertical()
                                .id_salt("store-app-picker")
                                .max_height(260.0)
                                .auto_shrink([false, false]),
                            |ui| {
                                let mut any = false;
                                for app in apps.iter().filter(|app| {
                                    needle.is_empty() || app.name.to_lowercase().contains(&needle)
                                }) {
                                    any = true;
                                    if ui
                                        .add(
                                            components::borderless_button(
                                                RichText::new(&app.name).color(theme::INK),
                                                ButtonRole::Secondary,
                                            )
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                        )
                                        .on_hover_text(&app.aumid)
                                        .clicked()
                                    {
                                        chosen = Some(app.aumid.clone());
                                    }
                                    ui.add_space(2.0);
                                }
                                if !any {
                                    ui.label(text::note(t!("gs.no_match")));
                                }
                            },
                        );
                    }
                    None => {}
                }
            });
            if let Some(aumid) = chosen {
                draft.config_targets.push(PathBuf::from(
                    crate::launcher_gui::store_apps::launch_path(&aumid),
                ));
                draft.store_picker = None;
                draft.store_filter.clear();
            } else if close {
                draft.store_picker = None;
                draft.store_filter.clear();
            }
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let draft = &*self.draft;
        let event = &mut *self.event;
        let valid = draft.close_mode != CloseMode::Script || draft.close_script.is_some();
        if components::action(ui, t!("common.cancel")).clicked() {
            *event = DialogEvent::Cancel;
        }
        if components::action_enabled(ui, valid, t!("common.save").to_string()).clicked() {
            *event = DialogEvent::SaveGameSettings {
                id: draft.id,
                watch_exe: draft.watch_override(),
                close_mode: draft.close_mode,
                close_script: draft.close_script.clone(),
                config_targets: draft.config_targets.clone(),
            };
        }
    }
}

fn show_plugin_settings(ui: &mut egui::Ui, draft: &mut GameSettingsDraft, event: &mut DialogEvent) {
    ui.label(RichText::new(t!("plugins.assignment").to_string()).strong());
    components::Combo::new("game-settings-plugin", &mut draft.plugin_choice.selected)
        .choice(None, t!("plugins.none"))
        .choices(
            draft
                .plugin_choice
                .plugins
                .iter()
                .enumerate()
                .map(|(index, plugin)| (Some(index), plugin_labels::plugin_choice_label(plugin))),
        )
        .placeholder(t!("plugins.none"))
        .full_width()
        .show(ui);
    if let Some(assignment) = &draft.plugin {
        ui.monospace(
            t!(
                "plugins.installed",
                id => assignment.plugin_id.as_str(),
                version => assignment.revision.version.to_string(),
                sha => assignment.revision.sha256.chars().take(12).collect::<String>()
            )
            .to_string(),
        );
        if let Some(path) = &assignment.resolved_launcher {
            ui.label(
                text::path(t!("plugins.launcher", path => path.display().to_string())).small(),
            );
        }
        if let Some(path) = &assignment.resolved_watch_target {
            ui.label(text::path(t!("plugins.watch", path => path.display().to_string())).small());
        }
        ui.label(text::hint(t!(
            "plugins.assignment_ledger",
            id => assignment.assignment_id.as_str()
        )));
    }
    show_assignment_inspection(ui, draft.plugin_inspection.as_ref());
    if let Some((plugin, options)) = draft.plugin_choice.selected_with_options() {
        super::plugin_options::show(
            ui,
            &plugin.manifest,
            super::plugin_options::OptionWording::GameSettings,
            options,
        );
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(selection) = draft.plugin_choice.selection() {
            let label = if draft.plugin.is_some() {
                t!("plugins.apply_change").to_string()
            } else {
                t!("plugins.install").to_string()
            };
            if components::action(ui, &label).clicked() {
                *event = DialogEvent::ApplyGamePlugin {
                    id: draft.id,
                    selection,
                };
            }
        }
        if components::action_enabled(ui, draft.plugin.is_some(), t!("plugins.repair").to_string())
            .clicked()
        {
            *event = DialogEvent::RepairGamePlugin(draft.id);
        }
        if components::action_enabled(
            ui,
            draft.plugin.is_some(),
            t!("plugins.reresolve").to_string(),
        )
        .clicked()
        {
            *event = DialogEvent::ReResolveGamePlugin(draft.id);
        }
        if ui
            .add_enabled(
                draft.plugin.is_some(),
                components::button(
                    t!("plugins.remove_assignment").to_string(),
                    ButtonRole::Destructive,
                ),
            )
            .clicked()
        {
            *event = DialogEvent::RemoveGamePlugin(draft.id);
        }
    });
    show_config_reset(ui, draft, event);
}

fn show_config_reset(
    ui: &mut egui::Ui,
    draft: &mut super::drafts::GameSettingsDraft,
    event: &mut DialogEvent,
) {
    let declares_config = match &draft.plugin_inspection {
        Some(PluginInspectionState::Loaded(inspection)) => inspection
            .active_files
            .iter()
            .any(|file| file.kind == ManagedFileKind::Config),
        _ => false,
    };
    if draft.plugin.is_none() {
        return;
    }
    ui.add_space(6.0);
    if draft.confirm_config_reset {
        ui.label(text::warning(t!("plugins.config_reset_confirm")));
        ui.horizontal(|ui| {
            if ui
                .add(components::button(
                    t!("plugins.config_reset_apply").to_string(),
                    ButtonRole::Destructive,
                ))
                .clicked()
            {
                draft.confirm_config_reset = false;
                *event = DialogEvent::ResetGamePluginConfig(draft.id);
            }
            if components::action(ui, t!("common.cancel")).clicked() {
                draft.confirm_config_reset = false;
            }
        });
        return;
    }
    if ui
        .add_enabled(
            declares_config,
            components::button(
                t!("plugins.config_reset").to_string(),
                ButtonRole::Secondary,
            ),
        )
        .on_hover_text(t!("plugins.config_reset_hint").to_string())
        .clicked()
    {
        draft.confirm_config_reset = true;
    }
}

fn show_assignment_inspection(ui: &mut egui::Ui, inspection: Option<&PluginInspectionState>) {
    use crate::plugin::deployment::FirstInstallOriginal;

    match inspection {
        Some(PluginInspectionState::Loading) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("plugins.inspection_loading").to_string());
            });
        }
        Some(PluginInspectionState::Failed(error)) => {
            ui.label(text::warning(t!(
                "plugins.inspection_failed",
                error => error.as_str()
            )));
        }
        Some(PluginInspectionState::Loaded(inspection)) => {
            if inspection.receipt_only {
                ui.label(text::warning(t!("plugins.receipt_only_warning")));
            }
            ui.monospace(
                t!("plugins.assignment_ledger", id => inspection.assignment_id.as_str())
                    .to_string(),
            );
            ui.label(
                t!(
                    "plugins.state_line",
                    state => plugin_labels::assignment_state_label(inspection.state),
                    transaction => inspection.latest_transaction_state
                        .map(plugin_labels::transaction_state_label)
                        .unwrap_or_else(|| t!("plugins.none").to_string()),
                    count => inspection.transaction_count
                )
                .to_string(),
            );
            ui.label(
                text::path(t!(
                    "plugins.recovery_receipt",
                    path => inspection.receipt_path.display().to_string()
                ))
                .small(),
            );
            egui::CollapsingHeader::new(
                t!(
                    "plugins.installed_files",
                    count => inspection.active_files.len()
                )
                .to_string(),
            )
            .default_open(true)
            .show(ui, |ui| {
                for file in &inspection.active_files {
                    ui.label(text::path(file.destination.display().to_string()).small());
                    let original = match &file.first_install_original {
                        FirstInstallOriginal::Existing { backup, .. } => {
                            t!("plugins.original_backup", path => backup.display().to_string())
                                .to_string()
                        }
                        FirstInstallOriginal::Absent => t!("plugins.originally_absent").to_string(),
                        FirstInstallOriginal::Preserved { .. } => {
                            t!("plugins.original_preserved").to_string()
                        }
                    };
                    ui.label(text::hint(original));
                    ui.label(text::hint(t!(
                        "plugins.backup_count",
                        count => file.backup_chain.len()
                    )));
                }
            });
            if !inspection.retired_files.is_empty() {
                egui::CollapsingHeader::new(
                    t!(
                        "plugins.retired_files",
                        count => inspection.retired_files.len()
                    )
                    .to_string(),
                )
                .show(ui, |ui| {
                    for file in &inspection.retired_files {
                        ui.label(text::path(file.destination.display().to_string()).small());
                    }
                });
            }
        }
        None => {}
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/game_settings__tests.rs"]
mod tests;
