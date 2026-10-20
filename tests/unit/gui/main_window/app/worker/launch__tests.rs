use std::path::PathBuf;

use super::*;
use crate::launcher_gui::model::{
    CloseMode, GameEntry, GameId, PluginAssignment, PluginRevision, PluginUpdatePolicy,
};
use crate::plugin::lifecycle::RevisionPolicy;

fn game(policy: PluginUpdatePolicy, allow_prerelease: bool) -> GameEntry {
    GameEntry {
        id: GameId(7),
        label: "Game".into(),
        controller_image: None,
        selected_target: PathBuf::from(r"C:\games\game.exe"),
        plugin: Some(PluginAssignment {
            plugin_id: "org.example.plugin".into(),
            revision: PluginRevision {
                version: semver::Version::parse("1.2.0").unwrap(),
                sha256: "f".repeat(64),
            },
            update_policy: policy,
            allow_prerelease,
            copy_zero_matches_to_game_folder: true,
            search_directory_destinations: false,
            assignment_id: "0123456789abcdef0123456789abcdef".into(),
            resolved_launcher: None,
            resolved_watch_target: None,
            grants: Vec::new(),
            ledger_reference: PathBuf::from(r"C:\state\ledger.toml"),
        }),
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

#[test]
fn a_launch_asks_for_exactly_the_pinned_revision_or_the_latest_stable_one() {
    let pinned = reconcile_request(&game(PluginUpdatePolicy::Pinned, true)).unwrap();
    assert_eq!(
        pinned.policy,
        RevisionPolicy::Pinned {
            version: semver::Version::parse("1.2.0").unwrap(),
            sha256: "f".repeat(64),
        },
        "a pinned game never leaves its revision, whatever its prerelease flag"
    );
    let tracking = reconcile_request(&game(PluginUpdatePolicy::TrackLatest, false)).unwrap();
    assert_eq!(
        tracking.policy,
        RevisionPolicy::TrackLatest {
            allow_prerelease: false
        }
    );
    assert_eq!(
        reconcile_request(&game(PluginUpdatePolicy::TrackLatest, true))
            .unwrap()
            .policy,
        RevisionPolicy::TrackLatest {
            allow_prerelease: true
        }
    );

    assert_eq!(tracking.assignment_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(tracking.plugin_id, "org.example.plugin");
    assert!(tracking.copy_zero_matches_to_game_folder);
    assert!(!tracking.search_directory_destinations);

    let mut plain = game(PluginUpdatePolicy::Pinned, false);
    plain.plugin = None;
    assert!(reconcile_request(&plain).is_err());
}
