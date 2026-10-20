use eframe::egui::{self, RichText};
use rust_i18n::t;

use crate::gui::common::components::{self, text, DialogContent};
use crate::plugin::deployment::{FirstInstallOriginal, ModificationDecision, ModificationPurpose};
use crate::plugin::paths::{AmbiguityKind, ManagedFileKind};
use crate::plugin::workflow::WorkflowAction;

use super::{plugin_labels, DialogEvent, PluginActionDraft};

pub(super) struct PluginActionDialog<'a> {
    draft: &'a mut PluginActionDraft,
    event: &'a mut DialogEvent,
}

impl<'a> PluginActionDialog<'a> {
    pub(super) fn new(draft: &'a mut PluginActionDraft, event: &'a mut DialogEvent) -> Self {
        Self { draft, event }
    }
}

impl DialogContent for PluginActionDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let preview = &draft.preview;
        ui.label(RichText::new(action_heading(preview.action)).strong());
        ui.label(&preview.description);
        ui.label(t!("plugins.plan.plugin_label", label => preview.label.as_str()).to_string());
        ui.monospace(
            t!(
                "plugins.plan.identity",
                id => preview.plugin_id.as_str(),
                version => preview.plugin_version.to_string(),
                md5 => preview.plugin_md5.as_str(),
                sha => preview.plugin_sha256.as_str()
            )
            .to_string(),
        );
        ui.label(
            text::path(t!(
                "plugins.plan.selected_target",
                path => preview.selected_target.display().to_string()
            ))
            .small(),
        );
        if let Some(state) = preview.assignment_state {
            ui.label(
                t!(
                    "plugins.plan.assignment",
                    state => plugin_labels::assignment_state_label(state),
                    transaction => preview
                        .latest_transaction_state
                        .map(plugin_labels::transaction_state_label)
                        .unwrap_or_else(|| t!("plugins.none").to_string()),
                    count => preview.transaction_count
                )
                .to_string(),
            );
        }

        show_ambiguities(ui, draft);
        show_install_scripts(ui, draft);
        show_files(ui, draft);
        show_initial_configuration_choices(ui, draft);
        show_external_grants(ui, draft);
        show_modifications(ui, draft);

        ui.add_space(12.0);
        ui.label(text::hint(t!("plugins.plan.no_mutation_before_approval")));
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let draft = &*self.draft;
        let event = &mut *self.event;
        if components::action(ui, t!("common.cancel")).clicked() {
            *event = draft.cancellation_event();
        }
        if components::action_enabled(ui, draft.is_complete(), confirm_label(draft.preview.action))
            .on_disabled_hover_text(t!("plugins.plan.incomplete").to_string())
            .clicked()
        {
            if let Some(confirmation) = draft.confirmation_event() {
                *event = confirmation;
            }
        }
    }
}

fn show_install_scripts(ui: &mut egui::Ui, draft: &PluginActionDraft) {
    if draft.preview.install_scripts.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            t!(
                "plugins.plan.install_scripts",
                count => draft.preview.install_scripts.len()
            )
            .to_string(),
        )
        .strong(),
    );
    ui.label(text::hint(t!("plugins.plan.install_scripts_warning")));
    for script in &draft.preview.install_scripts {
        components::card(ui, |ui| {
            ui.monospace(&script.source_relative);
            if !script.arguments.is_empty() {
                ui.label(
                    RichText::new(
                        t!(
                            "plugins.plan.install_script_arguments",
                            arguments => script.arguments.join(" ")
                        )
                        .to_string(),
                    )
                    .small(),
                );
            }
            if script.already_completed {
                ui.label(text::hint(t!("plugins.plan.install_script_completed")));
            }
        });
        ui.add_space(5.0);
    }
}

fn show_ambiguities(ui: &mut egui::Ui, draft: &mut PluginActionDraft) {
    if draft.preview.ambiguities.is_empty() {
        if let Some(path) = &draft.preview.resolved_launcher {
            ui.label(t!("plugins.launcher", path => path.display().to_string()).to_string());
        }
        if let Some(path) = &draft.preview.resolved_watch_target {
            ui.label(t!("plugins.watch", path => path.display().to_string()).to_string());
        }
        return;
    }
    ui.add_space(10.0);
    ui.label(text::warning(t!("plugins.plan.ambiguity_warning")));
    for ambiguity in draft.preview.ambiguities.clone() {
        let (label, selected) = match ambiguity.kind {
            AmbiguityKind::Launcher => (
                t!("plugins.plan.launcher_target").to_string(),
                &mut draft.approval.launcher_destination,
            ),
            AmbiguityKind::WatchTarget => (
                t!("plugins.plan.watch_target").to_string(),
                &mut draft.approval.watch_destination,
            ),
        };

        let unmatched = selected
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| t!("plugins.plan.choose_one").to_string());
        components::Combo::new(("plugin-plan-ambiguity", ambiguity.kind as u8), selected)
            .choices(ambiguity.candidates.into_iter().map(|candidate| {
                let label = candidate.display().to_string();
                (Some(candidate), label)
            }))
            .placeholder(unmatched)
            .full_width()
            .show(ui);
        ui.label(text::hint(label));
    }
}

fn show_files(ui: &mut egui::Ui, draft: &PluginActionDraft) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            t!("plugins.plan.managed_files", count => draft.preview.files.len()).to_string(),
        )
        .strong(),
    );
    if draft.preview.files.is_empty() {
        ui.label(text::note(t!("plugins.plan.no_destinations")));
        return;
    }
    components::scroll_area(
        ui,
        egui::ScrollArea::vertical()
            .id_salt("plugin-plan-files")
            .max_height(230.0)
            .auto_shrink([false, true]),
        |ui| {
            for file in &draft.preview.files {
                components::card(ui, |ui| {
                    let kind = match file.kind {
                        ManagedFileKind::Config => t!("plugins.plan.config").to_string(),
                        ManagedFileKind::Plugin => t!("plugins.plan.plugin").to_string(),
                    };
                    ui.label(RichText::new(format!("{kind}: {}", file.source_relative)).strong());
                    ui.label(text::path(file.destination.display().to_string()).small());
                    let mut facts = Vec::new();
                    if let Some(size) = file.source_size {
                        facts.push(t!("plugins.plan.bytes", count => size).to_string());
                    }

                    facts.push(match (file.kind, file.destination_existed) {
                        (ManagedFileKind::Config, true) => {
                            if overwrite_chosen(draft, &file.destination) {
                                t!("plugins.plan.config_overwritten").to_string()
                            } else {
                                t!("plugins.plan.config_kept").to_string()
                            }
                        }
                        (ManagedFileKind::Config, false) => {
                            t!("plugins.plan.config_created").to_string()
                        }
                        (ManagedFileKind::Plugin, true) => {
                            t!("plugins.plan.plugin_replaced").to_string()
                        }
                        (ManagedFileKind::Plugin, false) => {
                            t!("plugins.plan.new_destination").to_string()
                        }
                    });
                    if file.outside_game_folder {
                        facts.push(t!("plugins.plan.outside_game_folder").to_string());
                    }
                    if file.add_to_config_menu {
                        facts.push(t!("plugins.plan.shown_in_config_menu").to_string());
                    }
                    ui.label(text::hint(facts.join(" · ")));
                    if let Some(original) = &file.first_install_original {
                        match original {
                            FirstInstallOriginal::Existing { backup, .. } => {
                                ui.label(RichText::new(t!("plugins.plan.first_original", path => backup.display().to_string()).to_string()).small());
                            }
                            FirstInstallOriginal::Absent => {
                                ui.label(
                                    RichText::new(t!("plugins.plan.first_absent").to_string())
                                        .small(),
                                );
                            }
                            FirstInstallOriginal::Preserved { .. } => {
                                ui.label(
                                    RichText::new(t!("plugins.plan.first_preserved").to_string())
                                        .small(),
                                );
                            }
                        }
                    }
                    for backup in &file.backup_chain {
                        ui.label(
                                RichText::new(t!("plugins.plan.backup", path => backup.path.display().to_string(), reason => backup.reason.as_str()).to_string())
                                    .small(),
                            );
                    }
                });
                ui.add_space(5.0);
            }
        },
    );
}

fn overwrite_chosen(draft: &PluginActionDraft, destination: &std::path::Path) -> bool {
    draft
        .approval
        .modification_decisions
        .get(&crate::plugin::deployment::decision_key(destination))
        .copied()
        == Some(ModificationDecision::BackupAndOverwrite)
}

fn show_initial_configuration_choices(ui: &mut egui::Ui, draft: &mut PluginActionDraft) {
    if !matches!(
        draft.preview.action,
        WorkflowAction::AddGame | WorkflowAction::Install
    ) {
        return;
    }
    let existing: Vec<std::path::PathBuf> = draft
        .preview
        .files
        .iter()
        .filter(|file| file.kind == ManagedFileKind::Config && file.destination_existed)
        .map(|file| file.destination.clone())
        .collect();
    if existing.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            t!("plugins.plan.initial_config_heading", count => existing.len()).to_string(),
        )
        .strong(),
    );
    ui.label(text::hint(t!("plugins.plan.initial_config_intro")));
    for destination in existing {
        let key = crate::plugin::deployment::decision_key(&destination);
        let mut selected = draft
            .approval
            .modification_decisions
            .get(&key)
            .copied()
            .unwrap_or(ModificationDecision::KeepCurrent);
        components::card(ui, |ui| {
            ui.label(text::path(destination.display().to_string()));
            components::radio(
                ui,
                &mut selected,
                ModificationDecision::KeepCurrent,
                t!("plugins.plan.keep_current").to_string(),
            );
            components::radio(
                ui,
                &mut selected,
                ModificationDecision::BackupAndOverwrite,
                t!("plugins.plan.backup_and_overwrite").to_string(),
            );
        });
        draft.approval.modification_decisions.insert(key, selected);
        ui.add_space(5.0);
    }
}

fn show_external_grants(ui: &mut egui::Ui, draft: &mut PluginActionDraft) {
    if draft.preview.required_external_grants.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(text::warning(t!("plugins.plan.external_warning")));
    for grant in draft.preview.required_external_grants.clone() {
        let mut accepted = draft.external_granted(&grant.destination_root);
        if components::checkbox(
            ui,
            &mut accepted,
            t!("plugins.plan.allow_writes", path => grant.destination_root.display().to_string())
                .to_string(),
        )
        .changed()
        {
            draft.set_external_granted(grant.destination_root, accepted);
        }
    }
}

fn show_modifications(ui: &mut egui::Ui, draft: &mut PluginActionDraft) {
    if draft.preview.modifications.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(text::warning(t!("plugins.plan.modified_warning")));
    for modification in &draft.preview.modifications {
        let key = crate::plugin::deployment::decision_key(&modification.destination);
        let mut selected = draft.approval.modification_decisions.get(&key).copied();
        components::card(ui, |ui| {
            ui.label(text::path(modification.destination.display().to_string()));
            let current_hash = modification
                .current_sha256
                .as_deref()
                .map(short_hash)
                .map(str::to_owned)
                .unwrap_or_else(|| t!("plugins.plan.missing").to_string());
            ui.label(text::hint(t!(
                "plugins.plan.hashes",
                deployed => short_hash(&modification.last_deployed_sha256),
                current => current_hash
            )));
            components::radio(
                ui,
                &mut selected,
                Some(ModificationDecision::KeepCurrent),
                t!("plugins.plan.keep_current").to_string(),
            );
            match modification.purpose {
                ModificationPurpose::ReplaceWithPlugin => {
                    components::radio(
                        ui,
                        &mut selected,
                        Some(ModificationDecision::BackupAndOverwrite),
                        t!("plugins.plan.backup_and_overwrite").to_string(),
                    );
                }
                ModificationPurpose::RestoreOriginal => {
                    components::radio(
                        ui,
                        &mut selected,
                        Some(ModificationDecision::RestoreOriginal),
                        t!("plugins.plan.preserve_restore").to_string(),
                    );
                }
            }
        });
        match selected {
            Some(decision) => {
                draft.approval.modification_decisions.insert(key, decision);
            }
            None => {
                draft.approval.modification_decisions.remove(&key);
            }
        }
        ui.add_space(5.0);
    }
}

fn action_heading(action: WorkflowAction) -> String {
    match action {
        WorkflowAction::AddGame => t!("plugins.plan.action.add_game").to_string(),
        WorkflowAction::Install => t!("plugins.plan.action.install").to_string(),
        WorkflowAction::Update => t!("plugins.plan.action.update").to_string(),
        WorkflowAction::Change => t!("plugins.plan.action.change").to_string(),
        WorkflowAction::Repair => t!("plugins.plan.action.repair").to_string(),
        WorkflowAction::Remove => t!("plugins.plan.action.remove").to_string(),
    }
}

fn confirm_label(action: WorkflowAction) -> String {
    match action {
        WorkflowAction::Remove => t!("plugins.plan.approve_remove").to_string(),
        _ => t!("plugins.plan.approve").to_string(),
    }
}

fn short_hash(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/plugin_action__tests.rs"]
mod tests;
