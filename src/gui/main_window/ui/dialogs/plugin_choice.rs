use crate::launcher_gui::model::PluginAssignment;
use crate::plugin::catalog::library::{PluginDescriptor, PluginSelection};

use super::plugin_options::PluginOptions;

#[derive(Clone, Debug)]
pub struct PluginChoice {
    pub plugins: Vec<PluginDescriptor>,
    pub selected: Option<usize>,
    pub track_latest: bool,
    pub allow_prerelease: bool,
    pub copy_zero_matches_to_game_folder: bool,
    pub search_directory_destinations: bool,
}

impl PluginChoice {
    pub fn new(plugins: Vec<PluginDescriptor>) -> Self {
        Self {
            plugins,
            selected: None,
            track_latest: true,
            allow_prerelease: false,
            copy_zero_matches_to_game_folder: false,
            search_directory_destinations: false,
        }
    }

    pub fn for_assignment(
        plugins: Vec<PluginDescriptor>,
        assignment: Option<&PluginAssignment>,
    ) -> Self {
        let Some(assignment) = assignment else {
            return Self::new(plugins);
        };
        let selected = plugins.iter().position(|candidate| {
            candidate.revision.plugin_id == assignment.plugin_id
                && candidate.revision.plugin_version == assignment.revision.version
                && candidate
                    .revision
                    .package_sha256
                    .eq_ignore_ascii_case(&assignment.revision.sha256)
        });
        Self {
            plugins,
            selected,
            track_latest: assignment.update_policy.tracks_latest(),
            allow_prerelease: assignment.allow_prerelease,
            copy_zero_matches_to_game_folder: assignment.copy_zero_matches_to_game_folder,
            search_directory_destinations: assignment.search_directory_destinations,
        }
    }

    pub fn selected_plugin(&self) -> Option<&PluginDescriptor> {
        self.selected.and_then(|index| self.plugins.get(index))
    }

    pub(super) fn selected_with_options(
        &mut self,
    ) -> Option<(&PluginDescriptor, PluginOptions<'_>)> {
        let plugin = self.selected.and_then(|index| self.plugins.get(index))?;
        Some((
            plugin,
            PluginOptions {
                track_latest: &mut self.track_latest,
                copy_zero_matches_to_game_folder: &mut self.copy_zero_matches_to_game_folder,
                search_directory_destinations: &mut self.search_directory_destinations,
            },
        ))
    }

    pub fn selection(&self) -> Option<PluginSelection> {
        let descriptor = self.selected_plugin()?;
        let mut selection = PluginSelection::exact(descriptor);
        selection.track_latest = self.track_latest;
        selection.allow_prerelease = self.allow_prerelease;
        selection.copy_zero_matches_to_game_folder = self.copy_zero_matches_to_game_folder;
        selection.search_directory_destinations = self.search_directory_destinations;
        Some(selection)
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/plugin_choice__tests.rs"]
mod tests;
