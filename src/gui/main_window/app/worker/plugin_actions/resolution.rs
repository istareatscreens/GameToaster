use std::path::PathBuf;

use crate::launcher_gui::model::PluginAssignment;
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::paths::{DirectoryFallbackChoice, ResolutionChoices, ZeroMatchChoice};

pub(super) fn selection_resolution_choices(
    selection: &PluginSelection,
    rule_count: usize,
) -> ResolutionChoices {
    resolution_choices(
        rule_count,
        selection.copy_zero_matches_to_game_folder,
        selection.search_directory_destinations,
        None,
        None,
        Vec::new(),
    )
}

pub(super) fn assignment_resolution_choices(
    assignment: &PluginAssignment,
    rule_count: usize,
) -> ResolutionChoices {
    resolution_choices(
        rule_count,
        assignment.copy_zero_matches_to_game_folder,
        assignment.search_directory_destinations,
        assignment.resolved_launcher.clone(),
        assignment.resolved_watch_target.clone(),
        assignment.grants.clone(),
    )
}

pub(super) fn resolution_choices(
    rule_count: usize,
    copy_zero_matches_to_game_folder: bool,
    search_directory_destinations: bool,
    launcher_destination: Option<PathBuf>,
    watch_destination: Option<PathBuf>,
    external_grants: Vec<crate::plugin::paths::ExternalGrant>,
) -> ResolutionChoices {
    let mut choices = ResolutionChoices {
        launcher_destination,
        watch_destination,
        external_grants,
        ..ResolutionChoices::default()
    };
    for index in 0..rule_count {
        choices.zero_match.insert(
            index,
            if copy_zero_matches_to_game_folder {
                ZeroMatchChoice::CopyToGameFolder
            } else {
                ZeroMatchChoice::NoAction
            },
        );
        choices.directory_fallback.insert(
            index,
            if search_directory_destinations {
                DirectoryFallbackChoice::SearchDirectory
            } else {
                DirectoryFallbackChoice::NoAction
            },
        );
    }
    choices
}
