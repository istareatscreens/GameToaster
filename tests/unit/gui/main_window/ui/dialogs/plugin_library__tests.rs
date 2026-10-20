use super::super::harness::{self, plugin_descriptor};
use super::super::Dialog;
use super::*;

#[test]
fn plugins_offers_close_in_the_empty_unselected_and_selected_states() {
    let empty = PluginLibraryDraft::new(Vec::new(), None, &[]);
    let mut unselected = PluginLibraryDraft::new(vec![plugin_descriptor()], None, &[]);
    unselected.selected = None;
    let selected = PluginLibraryDraft::new(vec![plugin_descriptor()], None, &[]);
    assert_eq!(selected.selected, Some(0));
    for draft in [empty, unselected, selected] {
        let mut shown = harness::show(Dialog::PluginLibrary(draft));
        assert_eq!(shown.actions().len(), 1);
        assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
    }
}

#[test]
fn an_assigned_revision_cannot_be_removed_and_an_unassigned_one_goes_to_recovery() {
    use crate::launcher_gui::model::{PluginAssignment, PluginRevision, PluginUpdatePolicy};
    let _locale = crate::gui::common::harness::locale("en");
    let plugin = plugin_descriptor();
    let mut assigned_game = harness::game_entry();
    assigned_game.plugin = Some(PluginAssignment {
        plugin_id: plugin.revision.plugin_id.clone(),
        revision: PluginRevision {
            version: plugin.revision.plugin_version.clone(),

            sha256: plugin.revision.package_sha256.to_uppercase(),
        },
        update_policy: PluginUpdatePolicy::Pinned,
        allow_prerelease: false,
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        assignment_id: "0123456789abcdef0123456789abcdef".into(),
        resolved_launcher: None,
        resolved_watch_target: None,
        grants: Vec::new(),
        ledger_reference: std::path::PathBuf::from("C:/state/ledger.toml"),
    });
    let mut shown = harness::show(Dialog::PluginLibrary(PluginLibraryDraft::new(
        vec![plugin.clone()],
        None,
        std::slice::from_ref(&assigned_game),
    )));
    assert!(matches!(
        shown.click_text(&t!("plugins.remove_revision")),
        DialogEvent::None
    ));
    let Dialog::PluginLibrary(draft) = &shown.dialog else {
        panic!("the library stays open");
    };
    assert!(
        !draft.confirm_remove,
        "an assigned revision's remove action takes no click"
    );
    let unassign_first = t!("plugins.unassign_first").to_string();
    assert!(
        shown
            .hover_texts(&t!("plugins.remove_revision"))
            .contains(&unassign_first),
        "the disabled action says why on hover"
    );

    let mut shown = harness::show(Dialog::PluginLibrary(PluginLibraryDraft::new(
        vec![plugin.clone()],
        None,
        &[],
    )));
    assert!(matches!(
        shown.click_text(&t!("plugins.remove_revision")),
        DialogEvent::None
    ));
    let Dialog::PluginLibrary(draft) = &shown.dialog else {
        panic!("the library stays open");
    };
    assert!(
        draft.confirm_remove,
        "an unassigned revision asks for confirmation"
    );

    let _ = shown.idle();
    match shown.click_text(&t!("plugins.move_recovery")) {
        DialogEvent::RemoveCatalogRevision(removed) => {
            assert_eq!(
                removed.revision.package_sha256,
                plugin.revision.package_sha256
            );
        }
        other => panic!("{other:?}"),
    }
}
