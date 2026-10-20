use eframe::egui::{self, RichText};
use rust_i18n::t;

use crate::gui::common::components::{self, text, DialogContent};

use super::drafts::AddDraft;
use super::{plugin_labels, DialogEvent};

pub(super) struct AddDialog<'a> {
    draft: &'a mut AddDraft,
    event: &'a mut DialogEvent,

    submit: bool,
}

impl<'a> AddDialog<'a> {
    pub(super) fn new(draft: &'a mut AddDraft, event: &'a mut DialogEvent) -> Self {
        Self {
            draft,
            event,
            submit: false,
        }
    }

    fn valid(&self) -> bool {
        !self.draft.label.trim().is_empty()
    }

    fn continue_event(&self) -> DialogEvent {
        let mut completed = self.draft.clone();
        completed.label = completed.label.trim().to_string();
        DialogEvent::AddContinue(completed)
    }
}

impl DialogContent for AddDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let event = &mut *self.event;
        ui.label(RichText::new(t!("add.selected_exe").to_string()).strong());
        ui.label(text::path(draft.path.display().to_string()));
        ui.add_space(8.0);
        ui.label(RichText::new(t!("add.game_label").to_string()).strong());
        let editor = ui.add(
            egui::TextEdit::singleline(&mut draft.label)
                .desired_width(f32::INFINITY)
                .hint_text(t!("add.label_hint").to_string()),
        );
        components::focus_once(&editor, &mut draft.label_focused);
        ui.add_space(8.0);
        ui.label(RichText::new(t!("plugins.field").to_string()).strong());
        components::Combo::new("add-game-plugin", &mut draft.plugin_choice.selected)
            .choice(None, t!("plugins.none"))
            .choices(
                draft
                    .plugin_choice
                    .plugins
                    .iter()
                    .enumerate()
                    .map(|(index, plugin)| {
                        (Some(index), plugin_labels::plugin_choice_label(plugin))
                    }),
            )
            .placeholder(t!("plugins.none"))
            .full_width()
            .show(ui);
        if components::action(ui, t!("plugins.import").to_string()).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(t!("plugins.import_title").to_string())
                .add_filter(t!("plugins.package_filter").to_string(), &["zip", "toml"])
                .pick_file()
            {
                *event = DialogEvent::ImportPlugin(path);
            }
        }
        if let Some((plugin, options)) = draft.plugin_choice.selected_with_options() {
            ui.add_space(6.0);
            components::card(ui, |ui| {
                ui.label(RichText::new(&plugin.manifest.description).small());
                ui.label(text::hint(t!(
                    "plugins.identity",
                    id => plugin.revision.plugin_id.as_str(),
                    md5 => plugin.revision.package_md5.as_str(),
                    sha => plugin.digest_summary()
                )));
                super::plugin_options::show(
                    ui,
                    &plugin.manifest,
                    super::plugin_options::OptionWording::AddGame,
                    options,
                );
                let launcher = plugin
                    .manifest
                    .game_launcher
                    .as_deref()
                    .map(str::to_string)
                    .unwrap_or_else(|| t!("plugins.selected_game_target").to_string());
                let watch = plugin
                    .manifest
                    .executable_watch
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| t!("plugins.selected_target").to_string());
                ui.label(
                    text::path(t!(
                        "plugins.defaults",
                        launcher => launcher,
                        watch => watch,
                        count => plugin.manifest.config_files.len() + plugin.manifest.plugin_files.len()
                    ))
                    .small(),
                );
            });
        } else {
            ui.label(text::hint(t!("plugins.no_plugin")));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(t!("gs.watch_title").to_string()).strong());
        ui.label(text::hint(t!("add.watch_hint")));

        draft.sync_watch_default();
        let placeholder = draft
            .resolved_watch_name()
            .unwrap_or_else(|| t!("gs.watch_placeholder").to_string());
        super::watch_field::show(ui, &mut draft.watch_exe, &placeholder);
        self.submit = self.valid()
            && editor.lost_focus()
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("common.cancel")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
        let clicked = components::action_enabled(ui, self.valid(), t!("common.continue")).clicked();
        if clicked || self.submit {
            *self.event = self.continue_event();
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/add__tests.rs"]
mod tests;
