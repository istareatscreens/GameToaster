use super::*;
use crate::launcher_gui::model::{CloseMode, GameId, PluginRevision, PluginUpdatePolicy};
use crate::plugin::deployment::{
    BackupRecord, FirstInstallOriginal, RecoveryFile, TransactionState,
};

fn game_and_assignment() -> (GameEntry, PluginAssignment) {
    let assignment = PluginAssignment {
        plugin_id: "org.example.receipt".into(),
        revision: PluginRevision {
            version: semver::Version::parse("1.2.3").unwrap(),
            sha256: "a".repeat(64),
        },
        update_policy: PluginUpdatePolicy::Pinned,
        allow_prerelease: false,
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        assignment_id: "0123456789abcdef0123456789abcdef".into(),
        resolved_launcher: None,
        resolved_watch_target: None,
        grants: Vec::new(),
        ledger_reference: PathBuf::from(r"C:\state\ledger.toml"),
    };
    let game = GameEntry {
        id: GameId(7),
        label: "Receipt fixture".into(),
        controller_image: None,
        selected_target: PathBuf::from(r"C:\games\game.exe"),
        plugin: Some(assignment.clone()),
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    };
    (game, assignment)
}

fn receipt(assignment: &PluginAssignment) -> crate::plugin::deployment::RecoveryReceipt {
    crate::plugin::deployment::RecoveryReceipt {
        receipt_version: 1,
        assignment_id: assignment.assignment_id.clone(),
        plugin_id: assignment.plugin_id.clone(),
        plugin_version: assignment.revision.version.to_string(),
        revision_sha256: assignment.revision.sha256.clone(),
        game_id: 7,
        transaction_state: Some(crate::plugin::deployment::TransactionState::Committed),
        install_script_receipts: Vec::new(),
        files: vec![RecoveryFile {
            kind: crate::plugin::paths::ManagedFileKind::Plugin,
            destination: PathBuf::from(r"C:\games\plugin.bin"),
            first_install_original: FirstInstallOriginal::Existing {
                backup: PathBuf::from(r"C:\games\plugin.bin.20260727T010203.004Z.original"),
                sha256: "b".repeat(64),
            },
            last_deployed_sha256: "c".repeat(64),
            backups: vec![BackupRecord {
                path: PathBuf::from(r"C:\games\plugin.bin.20260727T010203.004Z.original"),
                sha256: "b".repeat(64),
                reason: "first-install".into(),
            }],
        }],
    }
}

#[test]
fn state_loss_inspection_is_receipt_only_and_never_claims_active_state() {
    let (game, assignment) = game_and_assignment();
    let path =
        PathBuf::from(r"C:\games\.GameToaster\0123456789abcdef0123456789abcdef\receipt.toml");
    let inspection =
        receipt_only_inspection(&game, &assignment, path.clone(), receipt(&assignment)).unwrap();
    assert!(inspection.receipt_only);
    assert_eq!(inspection.state, AssignmentState::RecoveryRequired);
    assert_eq!(
        inspection.latest_transaction_state,
        Some(TransactionState::Committed)
    );
    assert_eq!(inspection.transaction_count, 0);
    assert_eq!(inspection.receipt_path, path);
    assert_eq!(inspection.active_files.len(), 1);
    assert_eq!(
        inspection.active_files[0].destination,
        PathBuf::from(r"C:\games\plugin.bin")
    );
    assert_eq!(inspection.active_files[0].backup_chain.len(), 1);
}

#[test]
fn state_loss_inspection_rejects_a_receipt_for_another_revision() {
    let (game, assignment) = game_and_assignment();
    let mut wrong = receipt(&assignment);
    wrong.revision_sha256 = "d".repeat(64);
    assert!(receipt_only_inspection(
        &game,
        &assignment,
        PathBuf::from(r"C:\games\.GameToaster\receipt.toml"),
        wrong,
    )
    .unwrap_err()
    .contains("does not match"));
}
