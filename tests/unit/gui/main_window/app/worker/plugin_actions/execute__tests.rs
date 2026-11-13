use std::path::Path;

use super::*;
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::launcher_gui::model::{GameEntry, GameId, PluginUpdatePolicy};
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::catalog::{Catalog, CatalogRevision};
use crate::plugin::deployment::{AssignmentLedger, AssignmentState, DeploymentEngine};
use crate::plugin::lifecycle::LifecycleService;
use crate::plugin::paths::PathPolicy;

const ASSIGNMENT: &str = "0123456789abcdef0123456789abcdef";

fn backend(root: &Path) -> PluginBackend {
    let catalog_root = root.join("catalog");
    let state_root = root.join("state");
    std::fs::create_dir_all(&catalog_root).unwrap();
    let deployment = DeploymentEngine::new(
        &state_root,
        PathPolicy {
            application_root: root.to_path_buf(),
            catalog_root: catalog_root.clone(),
            state_root: state_root.clone(),
        },
    )
    .unwrap();
    let lifecycle =
        LifecycleService::new(Catalog::open(&catalog_root).unwrap(), deployment.clone()).unwrap();
    PluginBackend {
        catalog: Catalog::open(&catalog_root).unwrap(),
        deployment,
        lifecycle,
    }
}

#[test]
fn approval_stores_the_selections_policy_options_and_the_ledgers_resolved_paths() {
    let root = tempfile::tempdir().unwrap();
    let backend = backend(root.path());
    let game_folder = root.path().join("game");
    let revision = CatalogRevision {
        plugin_id: "org.example.plugin".into(),
        plugin_version: semver::Version::parse("1.2.0").unwrap(),
        package_md5: "a".repeat(32),
        package_sha256: "f".repeat(64),
        imported_at: 0,
        root: root.path().join("catalog").join("revision"),
    };
    let ledger = AssignmentLedger {
        ledger_version: 1,
        assignment_id: ASSIGNMENT.into(),
        plugin_id: "org.example.plugin".into(),
        plugin_version: "1.2.0".into(),
        revision_sha256: "f".repeat(64),
        game_id: 7,
        game_folder: game_folder.clone(),
        selected_target: game_folder.join("game.exe"),
        resolved_launcher: Some(game_folder.join("launcher.exe")),
        resolved_watch_target: Some(game_folder.join("patched.exe")),
        state: AssignmentState::Active,
        files: Vec::new(),
        retired_files: Vec::new(),
        transactions: Vec::new(),
        install_script_receipts: Vec::new(),
    };
    for track_latest in [false, true] {
        let selection = PluginSelection {
            plugin_id: "org.example.plugin".into(),
            version: semver::Version::parse("1.2.0").unwrap(),
            sha256: "f".repeat(64),
            track_latest,
            allow_prerelease: true,
            copy_zero_matches_to_game_folder: true,
            search_directory_destinations: false,
        };
        let mut game = GameEntry {
            id: GameId(7),
            label: "Game".into(),
            controller_image: None,
            selected_target: game_folder.join("game.exe"),
            plugin: None,
            hotkey: None,
            watch_exe: None,
            pre_start_script: None,
            close_script: None,
            close_hotkey: None,
            config_targets: Vec::new(),
        };
        apply_new_assignment(
            &mut game,
            &revision,
            &selection,
            ASSIGNMENT.into(),
            &ledger,
            Vec::new(),
            &backend,
        )
        .unwrap();
        let assignment = game.plugin.expect("the game is assigned");
        assert_eq!(
            assignment.update_policy,
            PluginUpdatePolicy::for_tracking(track_latest),
            "track_latest={track_latest}"
        );
        assert_eq!(
            assignment.update_policy == PluginUpdatePolicy::Pinned,
            !track_latest
        );
        assert_eq!(assignment.plugin_id, "org.example.plugin");
        assert_eq!(assignment.revision.version, revision.plugin_version);
        assert_eq!(assignment.revision.sha256, revision.package_sha256);
        assert!(assignment.allow_prerelease);
        assert!(assignment.copy_zero_matches_to_game_folder);
        assert!(!assignment.search_directory_destinations);
        assert_eq!(assignment.assignment_id, ASSIGNMENT);
        assert_eq!(assignment.resolved_launcher, ledger.resolved_launcher);
        assert_eq!(
            assignment.resolved_watch_target,
            ledger.resolved_watch_target
        );
        assert!(assignment.ledger_reference.ends_with("ledger.toml"));
        assert!(assignment.grants.is_empty());
    }
}

#[test]
fn a_listed_file_needs_no_answer_and_an_answer_that_does_not_fit_is_refused() {
    use crate::plugin::deployment::{decision_key, ModificationPurpose, UserModification};

    let _locale = crate::gui::common::harness::locale("en");
    let listed_for = |path: &str, purpose| UserModification {
        destination: path.into(),
        purpose,
        last_deployed_sha256: "c".repeat(64),
        current_sha256: Some("d".repeat(64)),
    };
    let replace = listed_for(r"C:\Games\hook.dll", ModificationPurpose::ReplaceWithPlugin);
    let restore = listed_for(
        r"C:\Games\retired.dll",
        ModificationPurpose::RestoreOriginal,
    );
    let listed = [replace.clone(), restore.clone()];
    let answer = |modification: &UserModification, decision| {
        BTreeMap::from([(decision_key(&modification.destination), decision)])
    };

    assert_eq!(
        validate_modification_approval(&listed, &BTreeMap::new()),
        Ok(())
    );

    for (modification, decision) in [
        (&replace, ModificationDecision::KeepCurrent),
        (&replace, ModificationDecision::BackupAndOverwrite),
        (&restore, ModificationDecision::KeepCurrent),
        (&restore, ModificationDecision::RestoreOriginal),
    ] {
        assert_eq!(
            validate_modification_approval(&listed, &answer(modification, decision)),
            Ok(()),
            "{decision:?} for {}",
            modification.destination.display()
        );
    }

    for (modification, decision) in [
        (&replace, ModificationDecision::RestoreOriginal),
        (&restore, ModificationDecision::BackupAndOverwrite),
    ] {
        assert_eq!(
            validate_modification_approval(&listed, &answer(modification, decision)),
            Err(format!(
                "approved action is invalid for {}",
                modification.destination.display()
            )),
            "{decision:?}"
        );
    }
}
