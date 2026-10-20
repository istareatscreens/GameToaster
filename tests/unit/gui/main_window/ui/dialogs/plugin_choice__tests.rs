use super::super::harness::plugin_descriptor;
use super::*;
use crate::launcher_gui::model::PluginUpdatePolicy;

fn choice(selected: Option<usize>) -> PluginChoice {
    PluginChoice {
        selected,
        ..PluginChoice::new(vec![plugin_descriptor()])
    }
}

fn assignment(sha256: &str) -> PluginAssignment {
    PluginAssignment {
        plugin_id: "org.example.patcher".into(),
        revision: crate::launcher_gui::model::PluginRevision {
            version: semver::Version::parse("1.2.0").unwrap(),
            sha256: sha256.into(),
        },
        update_policy: PluginUpdatePolicy::Pinned,
        allow_prerelease: true,
        copy_zero_matches_to_game_folder: true,
        search_directory_destinations: false,
        assignment_id: "0123456789abcdef0123456789abcdef".into(),
        resolved_launcher: None,
        resolved_watch_target: None,
        grants: Vec::new(),
        ledger_reference: std::path::PathBuf::from("C:/state/ledger.toml"),
    }
}

fn state(choice: &PluginChoice) -> (Option<usize>, bool, bool, bool, bool) {
    (
        choice.selected,
        choice.track_latest,
        choice.allow_prerelease,
        choice.copy_zero_matches_to_game_folder,
        choice.search_directory_destinations,
    )
}

#[test]
fn a_new_game_chooses_no_plugin_and_tracks_the_latest_version() {
    let choice = PluginChoice::new(vec![plugin_descriptor()]);
    assert_eq!(choice.plugins.len(), 1);
    assert_eq!(state(&choice), (None, true, false, false, false));
}

#[test]
fn an_assignment_selects_its_revision_and_keeps_its_options() {
    let plugins = || vec![plugin_descriptor()];

    let assigned = assignment(&"F".repeat(64));
    assert_eq!(
        state(&PluginChoice::for_assignment(plugins(), Some(&assigned))),
        (Some(0), false, true, true, false)
    );

    let removed = assignment(&"0".repeat(64));
    assert_eq!(
        state(&PluginChoice::for_assignment(plugins(), Some(&removed))),
        (None, false, true, true, false)
    );
    assert_eq!(
        state(&PluginChoice::for_assignment(plugins(), None)),
        state(&PluginChoice::new(plugins()))
    );
}

#[test]
fn the_options_panel_edits_the_choices_own_options() {
    let mut chosen = choice(Some(0));
    let expected = plugin_descriptor().revision.package_sha256;
    let (plugin, options) = chosen
        .selected_with_options()
        .expect("a plugin is selected");
    assert_eq!(plugin.revision.package_sha256, expected);

    *options.track_latest = false;
    *options.copy_zero_matches_to_game_folder = true;
    assert_eq!(
        (
            chosen.track_latest,
            chosen.allow_prerelease,
            chosen.copy_zero_matches_to_game_folder,
            chosen.search_directory_destinations,
        ),
        (false, false, true, false)
    );
    let (_, options) = chosen.selected_with_options().unwrap();
    *options.search_directory_destinations = true;
    assert_eq!(
        (
            chosen.copy_zero_matches_to_game_folder,
            chosen.search_directory_destinations,
        ),
        (true, true)
    );
}

#[test]
fn without_a_selected_plugin_there_is_no_panel_and_no_selection() {
    for selected in [None, Some(1)] {
        let mut unchosen = choice(selected);
        assert!(unchosen.selected_plugin().is_none(), "{selected:?}");
        assert!(unchosen.selected_with_options().is_none(), "{selected:?}");
        assert!(unchosen.selection().is_none(), "{selected:?}");
    }
}
