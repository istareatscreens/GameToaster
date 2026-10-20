use eframe::egui;
use rust_i18n::t;

use crate::gui::common::components;
use crate::plugin::manifest::{CopyType, Manifest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OptionWording {
    AddGame,
    GameSettings,
}

pub(super) struct PluginOptions<'a> {
    pub track_latest: &'a mut bool,
    pub copy_zero_matches_to_game_folder: &'a mut bool,
    pub search_directory_destinations: &'a mut bool,
}

pub(super) fn manifest_uses_find_and_replace(manifest: &Manifest) -> bool {
    let replaces = |copy_type: &CopyType| {
        matches!(
            copy_type,
            CopyType::FindAndReplace | CopyType::FindAndReplaceInDirectory
        )
    };
    manifest
        .config_files
        .iter()
        .any(|file| replaces(&file.config_file_copy_type))
        || manifest
            .plugin_files
            .iter()
            .any(|file| replaces(&file.plugin_file_copy_type))
}

pub(super) fn show(
    ui: &mut egui::Ui,
    manifest: &Manifest,
    wording: OptionWording,
    options: PluginOptions<'_>,
) {
    let pin = match wording {
        OptionWording::AddGame => t!("plugins.pin_revision"),
        OptionWording::GameSettings => t!("plugins.pin_exact"),
    }
    .to_string();
    ui.horizontal(|ui| {
        components::radio(
            ui,
            options.track_latest,
            true,
            t!("plugins.track_latest").to_string(),
        );
        components::radio(ui, options.track_latest, false, pin);
    });
    ui.label(components::text::hint(t!("plugins.policy_hint")));
    if manifest_uses_find_and_replace(manifest) {
        let (zero, directory) = match wording {
            OptionWording::AddGame => ("plugins.zero_copy", "plugins.directory_search"),
            OptionWording::GameSettings => {
                ("plugins.zero_copy_short", "plugins.directory_search_short")
            }
        };
        components::checkbox(
            ui,
            options.copy_zero_matches_to_game_folder,
            t!(zero).to_string(),
        );
        ui.label(components::text::hint(t!("plugins.zero_copy_hint")));
        components::checkbox(
            ui,
            options.search_directory_destinations,
            t!(directory).to_string(),
        );
        ui.label(components::text::hint(t!("plugins.directory_search_hint")));
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/plugin_options__tests.rs"]
mod tests;
