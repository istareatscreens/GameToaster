use std::collections::BTreeMap;
use std::fs;

use super::*;
use crate::plugin::catalog::inventory::sha256_file;
use crate::plugin::manifest::{ConfigFile, Manifest, PluginFile};
use crate::plugin::paths::{build_plan, DeploymentPlan, PathPolicy, ResolutionChoices};
use crate::support::paths::paths_equal;
use semver::Version;

struct Fixture {
    _root: tempfile::TempDir,
    package: PathBuf,
    game: PathBuf,
    policy: PathPolicy,
    manifest: Manifest,
    plan: DeploymentPlan,
    engine: DeploymentEngine,
}

fn fixture(destination_exists: bool) -> Fixture {
    let root = crate::plugin::fixtures::canonical_tempdir();
    let package = root.path().join("package");
    let game = root.path().join("game");
    let state = root.path().join("state");
    fs::create_dir_all(&package).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("game.exe"), b"game").unwrap();
    if destination_exists {
        fs::write(game.join("plugin.bin"), b"original").unwrap();
    }
    fs::write(package.join("plugin.bin"), b"plugin-v1").unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: "org.example.transaction".into(),
        plugin_version: Version::parse("1.0.0").unwrap(),
        label: "Transaction".into(),
        description: "Transaction fixture".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: Vec::new(),
        plugin_files: vec![PluginFile {
            plugin_file_location: "plugin.bin".into(),
            plugin_file_copy_type: crate::plugin::manifest::CopyType::GameFolder,
            plugin_file_replace_path: None,
        }],
        install_scripts: Vec::new(),
    };
    let policy = PathPolicy {
        application_root: root.path().join("app"),
        catalog_root: root.path().join("catalog"),
        state_root: state.clone(),
    };
    let plan = build_plan(
        &package,
        &manifest,
        &"a".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy,
    )
    .unwrap();
    let engine = DeploymentEngine::new(&state, policy.clone()).unwrap();
    Fixture {
        _root: root,
        package,
        game,
        policy,
        manifest,
        plan,
        engine,
    }
}

fn replacement_plan(fixture: &Fixture) -> DeploymentPlan {
    let package = fixture.package.with_file_name("package-b");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("plugin.bin"), b"plugin-b").unwrap();
    let mut manifest = fixture.manifest.clone();
    manifest.plugin_id = "org.example.replacement".into();
    manifest.plugin_version = Version::parse("2.0.0").unwrap();
    fs::write(
        package.join("manifest.toml"),
        manifest.canonical_toml().unwrap(),
    )
    .unwrap();
    build_plan(
        &package,
        &manifest,
        &"c".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap()
}

fn multi_file_fixture() -> Fixture {
    let mut fixture = fixture(true);
    fixture.manifest.plugin_files.clear();
    for index in 1..=3 {
        let name = format!("managed-{index}.bin");
        fs::write(
            fixture.package.join(&name),
            format!("plugin-{index}").as_bytes(),
        )
        .unwrap();
        fs::write(
            fixture.game.join(&name),
            format!("original-{index}").as_bytes(),
        )
        .unwrap();
        fixture.manifest.plugin_files.push(PluginFile {
            plugin_file_location: name,
            plugin_file_copy_type: crate::plugin::manifest::CopyType::GameFolder,
            plugin_file_replace_path: None,
        });
    }
    fixture.plan = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"e".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    fixture
}

#[test]
fn install_update_and_remove_preserve_first_original_and_all_backups() {
    let mut fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-v1"
    );
    let FirstInstallOriginal::Existing { backup, .. } = &installed.files[0].first_install_original
    else {
        panic!("original was not recorded")
    };
    assert_eq!(fs::read(backup).unwrap(), b"original");

    fs::write(fixture.package.join("plugin.bin"), b"plugin-v2").unwrap();
    fixture.plan.files[0].source_sha256 = sha256_file(&fixture.package.join("plugin.bin")).unwrap();
    fixture.plan = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"b".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    let updated = fixture
        .engine
        .deploy(
            &assignment,
            "2.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(updated.files[0].backup_chain.len(), 2);
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-v2"
    );

    let removed = fixture
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(removed.state, AssignmentState::Removed);
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"original"
    );
    assert!(removed.transactions.last().unwrap().files[0]
        .transaction_backup
        .as_ref()
        .is_some_and(|path| path.exists()));
}

#[test]
fn originally_absent_file_moves_to_recovery_on_remove() {
    let fixture = fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert!(matches!(
        installed.files[0].first_install_original,
        FirstInstallOriginal::Absent
    ));
    let removed = fixture
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert!(!fixture.game.join("plugin.bin").exists());
    let recovery = removed.transactions.last().unwrap().files[0]
        .recovery_path
        .as_ref()
        .unwrap();
    assert_eq!(fs::read(recovery).unwrap(), b"plugin-v1");
}

#[test]
fn failed_or_uncertain_patcher_restore_performs_no_generic_cleanup() {
    let fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(fixture.game.join("plugin.bin"), b"user-edit").unwrap();
    let edited_hash = sha256_file(&fixture.game.join("plugin.bin")).unwrap();
    let transaction_count = installed.transactions.len();
    let original_backup = match &installed.files[0].first_install_original {
        FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
        FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
            panic!("fixture must have an original")
        }
    };

    let error = fixture
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Err("restore result is uncertain".into()),
            FailureInjection::default(),
        )
        .unwrap_err();
    assert!(error.contains("no managed files were changed"), "{error}");
    assert_eq!(
        sha256_file(&fixture.game.join("plugin.bin")).unwrap(),
        edited_hash
    );
    assert_eq!(fs::read(&original_backup).unwrap(), b"original");
    let ledger = fixture.engine.store().load(1, &assignment).unwrap();
    assert_eq!(ledger.state, AssignmentState::Active);
    assert_eq!(ledger.transactions.len(), transaction_count);
}

#[test]
fn user_modification_requires_decision_and_is_backed_up() {
    let mut fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(fixture.game.join("plugin.bin"), b"user-edit").unwrap();
    fixture.plan = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"a".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    let deployment_preview = fixture
        .engine
        .deployment_modifications(Some(&installed), &fixture.plan)
        .unwrap();
    assert_eq!(deployment_preview.len(), 1);
    assert_eq!(
        deployment_preview[0].purpose,
        ModificationPurpose::ReplaceWithPlugin
    );
    assert_eq!(
        deployment_preview[0].current_sha256.as_deref(),
        Some(
            sha256_file(&fixture.game.join("plugin.bin"))
                .unwrap()
                .as_str()
        )
    );
    let removal_preview = fixture.engine.removal_modifications(&installed).unwrap();
    assert_eq!(removal_preview.len(), 1);
    assert_eq!(
        removal_preview[0].purpose,
        ModificationPurpose::RestoreOriginal
    );
    assert!(fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap_err()
        .contains("modified by the user"));
    let mut decisions = BTreeMap::new();
    decisions.insert(
        decision_key(&fixture.game.join("plugin.bin")),
        ModificationDecision::BackupAndOverwrite,
    );
    let updated = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &decisions,
            FailureInjection::default(),
        )
        .unwrap();
    let last = updated.files[0].backup_chain.last().unwrap();
    assert_eq!(fs::read(&last.path).unwrap(), b"user-edit");
}

#[test]
fn modified_removal_requires_keep_or_restore_and_preserves_edited_bytes() {
    let keep = fixture(true);
    let assignment = keep.engine.new_assignment_id().unwrap();
    let installed = keep
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &keep.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(keep.game.join("plugin.bin"), b"user-edit-keep").unwrap();
    let transaction_count = installed.transactions.len();
    assert!(keep
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap_err()
        .contains("explicit keep/restore decision required"));
    assert_eq!(
        keep.engine
            .store()
            .load(1, &assignment)
            .unwrap()
            .transactions
            .len(),
        transaction_count
    );
    assert_eq!(
        fs::read(keep.game.join("plugin.bin")).unwrap(),
        b"user-edit-keep"
    );

    let mut invalid = BTreeMap::new();
    invalid.insert(
        decision_key(&keep.game.join("plugin.bin")),
        ModificationDecision::BackupAndOverwrite,
    );
    assert!(keep
        .engine
        .remove(
            1,
            &assignment,
            &invalid,
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap_err()
        .contains("not a valid removal decision"));
    assert_eq!(
        fs::read(keep.game.join("plugin.bin")).unwrap(),
        b"user-edit-keep"
    );

    let mut decisions = BTreeMap::new();
    decisions.insert(
        decision_key(&keep.game.join("plugin.bin")),
        ModificationDecision::KeepCurrent,
    );
    let removed = keep
        .engine
        .remove(
            1,
            &assignment,
            &decisions,
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(removed.state, AssignmentState::Removed);
    assert_eq!(
        fs::read(keep.game.join("plugin.bin")).unwrap(),
        b"user-edit-keep"
    );

    let restore = fixture(true);
    let assignment = restore.engine.new_assignment_id().unwrap();
    restore
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &restore.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(restore.game.join("plugin.bin"), b"user-edit-before-restore").unwrap();
    let mut decisions = BTreeMap::new();
    decisions.insert(
        decision_key(&restore.game.join("plugin.bin")),
        ModificationDecision::RestoreOriginal,
    );
    let removed = restore
        .engine
        .remove(
            1,
            &assignment,
            &decisions,
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(
        fs::read(restore.game.join("plugin.bin")).unwrap(),
        b"original"
    );
    let edit_backup = removed.transactions.last().unwrap().files[0]
        .transaction_backup
        .as_ref()
        .unwrap();
    assert_eq!(fs::read(edit_backup).unwrap(), b"user-edit-before-restore");
}

#[test]
fn update_can_retire_a_destination_without_destroying_provenance() {
    for destination_existed in [true, false] {
        let mut fixture = fixture(destination_existed);
        let assignment = fixture.engine.new_assignment_id().unwrap();
        fixture
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &fixture.plan,
                &BTreeMap::new(),
                FailureInjection::default(),
            )
            .unwrap();
        fixture.manifest.plugin_files.clear();
        fixture.manifest.plugin_version = Version::parse("2.0.0").unwrap();
        let empty_plan = build_plan(
            &fixture.package,
            &fixture.manifest,
            &"d".repeat(64),
            1,
            &fixture.game.join("game.exe"),
            &ResolutionChoices::default(),
            &fixture.policy,
        )
        .unwrap();
        let updated = fixture
            .engine
            .deploy(
                &assignment,
                "2.0.0",
                &empty_plan,
                &BTreeMap::new(),
                FailureInjection::default(),
            )
            .unwrap();
        assert!(updated.files.is_empty());
        assert_eq!(updated.retired_files.len(), 1);
        assert!(!updated.retired_files[0].backup_chain.is_empty());
        if destination_existed {
            assert_eq!(
                fs::read(fixture.game.join("plugin.bin")).unwrap(),
                b"original"
            );
        } else {
            assert!(!fixture.game.join("plugin.bin").exists());
            assert!(updated.transactions.last().unwrap().files[0]
                .recovery_path
                .as_ref()
                .is_some_and(|path| path.exists()));
        }
    }
}

#[test]
fn injected_failures_roll_back_every_transition_without_deleting_backups() {
    for point in [
        FailurePoint::AfterPlanned,
        FailurePoint::AfterStaged(0),
        FailurePoint::AfterBackedUp(0),
        FailurePoint::AfterCommitting,
        FailurePoint::AfterCommitted(0),
    ] {
        let fixture = fixture(true);
        let assignment = fixture.engine.new_assignment_id().unwrap();
        let error = fixture
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &fixture.plan,
                &BTreeMap::new(),
                FailureInjection {
                    fail: point,
                    rollback_at: None,
                },
            )
            .unwrap_err();
        assert!(error.contains("rolled back"), "{point:?}: {error}");
        assert_eq!(
            fs::read(fixture.game.join("plugin.bin")).unwrap(),
            b"original",
            "{point:?}"
        );
        let ledger = fixture.engine.store().load(1, &assignment).unwrap();
        assert_eq!(
            ledger.transactions.last().unwrap().state,
            TransactionState::RolledBack
        );
        for file in &ledger.transactions.last().unwrap().files {
            if let Some(backup) = &file.transaction_backup {
                assert!(backup.exists());
            }
        }
    }
}

#[test]
fn every_multi_file_partial_commit_rolls_back_all_bytes_and_retains_backups() {
    for committed_index in 0..3 {
        let fixture = multi_file_fixture();
        let assignment = fixture.engine.new_assignment_id().unwrap();
        let point = FailurePoint::AfterCommitted(committed_index);
        let error = fixture
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &fixture.plan,
                &BTreeMap::new(),
                FailureInjection {
                    fail: point,
                    rollback_at: None,
                },
            )
            .unwrap_err();
        assert!(error.contains("rolled back"), "{point:?}: {error}");
        for index in 1..=3 {
            assert_eq!(
                fs::read(fixture.game.join(format!("managed-{index}.bin"))).unwrap(),
                format!("original-{index}").as_bytes(),
                "{point:?}: destination {index} did not return to its exact pre-transaction bytes"
            );
        }
        let ledger = fixture.engine.store().load(1, &assignment).unwrap();
        let transaction = ledger.transactions.last().unwrap();
        assert_eq!(transaction.state, TransactionState::RolledBack, "{point:?}");
        assert_eq!(transaction.files.len(), 3);
        assert!(transaction
            .files
            .iter()
            .all(|file| file.state == FileCommitState::RolledBack));
        assert!(transaction.files.iter().all(|file| file
            .transaction_backup
            .as_ref()
            .is_some_and(|backup| backup.is_file())));
    }
}

#[test]
fn restart_recovery_covers_every_install_update_and_remove_transition() {
    let transitions = [
        FailurePoint::AfterPlanned,
        FailurePoint::AfterStaged(0),
        FailurePoint::AfterBackedUp(0),
        FailurePoint::AfterCommitting,
        FailurePoint::AfterCommitted(0),
    ];

    for point in transitions {
        let install = fixture(true);
        let assignment = install.engine.new_assignment_id().unwrap();
        let error = install
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &install.plan,
                &BTreeMap::new(),
                FailureInjection {
                    fail: point,
                    rollback_at: Some(0),
                },
            )
            .unwrap_err();
        assert!(
            error.contains("requires recovery"),
            "install {point:?}: {error}"
        );
        install.engine.recover_all().unwrap();
        assert_eq!(
            fs::read(install.game.join("plugin.bin")).unwrap(),
            b"original",
            "install {point:?}"
        );
        let recovered = install.engine.store().load(1, &assignment).unwrap();
        assert_eq!(
            recovered.state,
            AssignmentState::Removed,
            "install {point:?}"
        );
        assert_eq!(
            recovered.transactions.last().unwrap().state,
            TransactionState::RolledBack,
            "install {point:?}"
        );
        assert_transaction_backups_retained(&recovered, point, "install");

        let mut update = fixture(true);
        let assignment = update.engine.new_assignment_id().unwrap();
        let installed = update
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &update.plan,
                &BTreeMap::new(),
                FailureInjection::default(),
            )
            .unwrap();
        let original_backup = match &installed.files[0].first_install_original {
            FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
            FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
                panic!("fixture must have an original")
            }
        };
        fs::write(update.package.join("plugin.bin"), b"plugin-v2").unwrap();
        update.plan = build_plan(
            &update.package,
            &update.manifest,
            &"8".repeat(64),
            1,
            &update.game.join("game.exe"),
            &ResolutionChoices::default(),
            &update.policy,
        )
        .unwrap();
        let error = update
            .engine
            .deploy(
                &assignment,
                "2.0.0",
                &update.plan,
                &BTreeMap::new(),
                FailureInjection {
                    fail: point,
                    rollback_at: Some(0),
                },
            )
            .unwrap_err();
        assert!(
            error.contains("requires recovery"),
            "update {point:?}: {error}"
        );
        update.engine.recover_all().unwrap();
        assert_eq!(
            fs::read(update.game.join("plugin.bin")).unwrap(),
            b"plugin-v1",
            "update {point:?}"
        );
        assert_eq!(
            fs::read(&original_backup).unwrap(),
            b"original",
            "update {point:?}"
        );
        let recovered = update.engine.store().load(1, &assignment).unwrap();
        assert_eq!(recovered.state, AssignmentState::Active, "update {point:?}");
        assert_eq!(
            recovered.transactions.last().unwrap().state,
            TransactionState::RolledBack,
            "update {point:?}"
        );
        assert_transaction_backups_retained(&recovered, point, "update");

        let remove = fixture(true);
        let assignment = remove.engine.new_assignment_id().unwrap();
        let installed = remove
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &remove.plan,
                &BTreeMap::new(),
                FailureInjection::default(),
            )
            .unwrap();
        let original_backup = match &installed.files[0].first_install_original {
            FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
            FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
                panic!("fixture must have an original")
            }
        };
        let error = remove
            .engine
            .remove(
                1,
                &assignment,
                &BTreeMap::new(),
                || Ok(()),
                FailureInjection {
                    fail: point,
                    rollback_at: Some(0),
                },
            )
            .unwrap_err();
        assert!(
            error.contains("requires recovery"),
            "remove {point:?}: {error}"
        );
        remove.engine.recover_all().unwrap();
        assert_eq!(
            fs::read(remove.game.join("plugin.bin")).unwrap(),
            b"plugin-v1",
            "remove {point:?}"
        );
        assert_eq!(
            fs::read(&original_backup).unwrap(),
            b"original",
            "remove {point:?}"
        );
        let recovered = remove.engine.store().load(1, &assignment).unwrap();
        assert_eq!(recovered.state, AssignmentState::Active, "remove {point:?}");
        assert_eq!(
            recovered.transactions.last().unwrap().state,
            TransactionState::RolledBack,
            "remove {point:?}"
        );
        assert_transaction_backups_retained(&recovered, point, "remove");
    }
}

fn assert_transaction_backups_retained(
    ledger: &AssignmentLedger,
    point: FailurePoint,
    operation: &str,
) {
    for transaction in &ledger.transactions {
        for file in &transaction.files {
            if let Some(backup) = &file.transaction_backup {
                assert!(
                    backup.is_file(),
                    "{operation} {point:?}: transaction backup disappeared: {}",
                    backup.display()
                );
            }
        }
    }
    for file in ledger.files.iter().chain(&ledger.retired_files) {
        if let FirstInstallOriginal::Existing { backup, .. } = &file.first_install_original {
            assert!(
                backup.is_file(),
                "{operation} {point:?}: first-install backup disappeared: {}",
                backup.display()
            );
        }
        for backup in &file.backup_chain {
            assert!(
                backup.path.is_file(),
                "{operation} {point:?}: provenance backup disappeared: {}",
                backup.path.display()
            );
        }
    }
}

#[test]
fn interrupted_rollback_marks_recovery_and_startup_finishes_it() {
    let fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let error = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: Some(0),
            },
        )
        .unwrap_err();
    assert!(error.contains("requires recovery"));
    assert_eq!(
        fixture.engine.store().load(1, &assignment).unwrap().state,
        AssignmentState::RecoveryRequired
    );
    fixture.engine.recover_all().unwrap();
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"original"
    );
    let ledger = fixture.engine.store().load(1, &assignment).unwrap();
    assert_eq!(
        ledger.transactions.last().unwrap().state,
        TransactionState::RolledBack
    );
}

#[test]
fn interrupted_install_update_and_remove_recover_without_byte_loss() {
    let install = fixture(true);
    let install_assignment = install.engine.new_assignment_id().unwrap();
    install
        .engine
        .deploy(
            &install_assignment,
            "1.0.0",
            &install.plan,
            &BTreeMap::new(),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: Some(0),
            },
        )
        .unwrap_err();
    install.engine.recover_all().unwrap();
    assert_eq!(
        fs::read(install.game.join("plugin.bin")).unwrap(),
        b"original"
    );

    let mut update = fixture(true);
    let update_assignment = update.engine.new_assignment_id().unwrap();
    let installed = update
        .engine
        .deploy(
            &update_assignment,
            "1.0.0",
            &update.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let original_backup = match &installed.files[0].first_install_original {
        FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
        FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
            panic!("fixture must have an original")
        }
    };
    fs::write(update.package.join("plugin.bin"), b"plugin-v2").unwrap();
    update.plan = build_plan(
        &update.package,
        &update.manifest,
        &"8".repeat(64),
        1,
        &update.game.join("game.exe"),
        &ResolutionChoices::default(),
        &update.policy,
    )
    .unwrap();
    update
        .engine
        .deploy(
            &update_assignment,
            "2.0.0",
            &update.plan,
            &BTreeMap::new(),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: Some(0),
            },
        )
        .unwrap_err();
    update.engine.recover_all().unwrap();
    assert_eq!(
        fs::read(update.game.join("plugin.bin")).unwrap(),
        b"plugin-v1"
    );
    assert_eq!(fs::read(&original_backup).unwrap(), b"original");

    let remove = fixture(true);
    let remove_assignment = remove.engine.new_assignment_id().unwrap();
    let installed = remove
        .engine
        .deploy(
            &remove_assignment,
            "1.0.0",
            &remove.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let original_backup = match &installed.files[0].first_install_original {
        FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
        FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
            panic!("fixture must have an original")
        }
    };
    remove
        .engine
        .remove(
            1,
            &remove_assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: Some(0),
            },
        )
        .unwrap_err();
    remove.engine.recover_all().unwrap();
    assert_eq!(
        fs::read(remove.game.join("plugin.bin")).unwrap(),
        b"plugin-v1"
    );
    assert_eq!(fs::read(&original_backup).unwrap(), b"original");
    assert_eq!(
        remove
            .engine
            .store()
            .load(1, &remove_assignment)
            .unwrap()
            .state,
        AssignmentState::Active
    );
}

#[test]
fn receipt_contains_only_metadata_and_survives_state_loss() {
    let fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let receipt = fixture
        .game
        .join(".GameToaster")
        .join(&assignment)
        .join("receipt.toml");
    let raw = fs::read_to_string(&receipt).unwrap();
    assert!(raw.contains("revisionSha256"));
    assert!(!raw.contains("plugin-v1"));
    let active_hash = sha256_file(&fixture.game.join("plugin.bin")).unwrap();
    let original_backup = match &installed.files[0].first_install_original {
        FirstInstallOriginal::Existing { backup, .. } => backup.clone(),
        FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => {
            panic!("fixture must have an original")
        }
    };
    let original_hash = sha256_file(&original_backup).unwrap();

    fs::remove_dir_all(&fixture.policy.state_root).unwrap();
    let parsed = LedgerStore::load_receipt(&receipt).unwrap();
    assert_eq!(parsed.assignment_id, assignment);
    assert_eq!(parsed.files.len(), 1);
    assert_eq!(parsed.files[0].destination, fixture.game.join("plugin.bin"));
    assert_eq!(parsed.files[0].last_deployed_sha256, active_hash);
    let FirstInstallOriginal::Existing { backup, sha256 } = &parsed.files[0].first_install_original
    else {
        panic!("receipt lost original provenance")
    };
    assert_eq!(backup, &original_backup);
    assert_eq!(sha256, &original_hash);
    assert_eq!(sha256_file(backup).unwrap(), original_hash);
    assert_eq!(
        sha256_file(&fixture.game.join("plugin.bin")).unwrap(),
        active_hash
    );
    let mut invalid = parsed;
    invalid.revision_sha256 = "not-a-hash".into();
    assert!(invalid.validate().is_err());
}

#[test]
fn config_targets_follow_only_committed_active_state() {
    let mut fixture = fixture(true);
    fixture.plan.files[0].add_to_config_menu = true;
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(
        installed.config_targets(),
        vec![fixture.game.join("plugin.bin")]
    );

    fs::write(fixture.package.join("plugin.bin"), b"plugin-v2").unwrap();
    let without_config = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"9".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    fixture
        .engine
        .deploy(
            &assignment,
            "2.0.0",
            &without_config,
            &BTreeMap::new(),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: None,
            },
        )
        .unwrap_err();
    assert_eq!(
        fixture
            .engine
            .store()
            .load(1, &assignment)
            .unwrap()
            .config_targets(),
        vec![fixture.game.join("plugin.bin")]
    );

    let without_config = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"9".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    let updated = fixture
        .engine
        .deploy(
            &assignment,
            "2.0.0",
            &without_config,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert!(updated.config_targets().is_empty());

    let removed = fixture
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert!(removed.config_targets().is_empty());
}

#[test]
fn plugin_change_commits_b_only_after_a_is_restored() {
    let mut fixture = fixture(true);
    fixture.plan.files[0].add_to_config_menu = true;
    let old_assignment = fixture.engine.new_assignment_id().unwrap();
    fixture
        .engine
        .deploy(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let mut new_plan = replacement_plan(&fixture);
    new_plan.files[0].add_to_config_menu = true;
    let new_assignment = fixture.engine.new_assignment_id().unwrap();
    let changed = fixture
        .engine
        .change_plugin(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            || Ok(()),
            || Ok(()),
            &new_assignment,
            "2.0.0",
            &new_plan,
            &BTreeMap::new(),
            || Ok(()),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(changed.removed_assignment.state, AssignmentState::Removed);
    assert_eq!(changed.active_assignment.state, AssignmentState::Active);
    assert!(changed.removed_assignment.config_targets().is_empty());
    assert_eq!(
        changed.active_assignment.config_targets(),
        vec![fixture.game.join("plugin.bin")]
    );
    assert_eq!(
        changed.active_assignment.transactions.last().unwrap().kind,
        TransactionKind::Change
    );
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-b"
    );
}

#[test]
fn failed_plugin_change_reinstalls_a_and_leaves_b_uncommitted() {
    let fixture = fixture(true);
    let old_assignment = fixture.engine.new_assignment_id().unwrap();
    fixture
        .engine
        .deploy(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let new_plan = replacement_plan(&fixture);
    let new_assignment = fixture.engine.new_assignment_id().unwrap();
    let error = fixture
        .engine
        .change_plugin(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            || Ok(()),
            || Ok(()),
            &new_assignment,
            "2.0.0",
            &new_plan,
            &BTreeMap::new(),
            || Ok(()),
            || Ok(()),
            FailureInjection {
                fail: FailurePoint::AfterCommitted(0),
                rollback_at: None,
            },
        )
        .unwrap_err();
    assert!(error.old_plugin_reinstalled, "{error}");
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-v1"
    );
    assert_eq!(
        fixture
            .engine
            .store()
            .load(1, &old_assignment)
            .unwrap()
            .state,
        AssignmentState::Active
    );
    let failed_b = fixture.engine.store().load(1, &new_assignment).unwrap();
    assert_eq!(failed_b.state, AssignmentState::Removed);
    assert_eq!(
        failed_b.transactions.last().unwrap().state,
        TransactionState::RolledBack
    );
}

#[test]
fn failed_replacement_prepare_restores_b_then_reinstalls_a() {
    let fixture = fixture(true);
    let old_assignment = fixture.engine.new_assignment_id().unwrap();
    fixture
        .engine
        .deploy(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    let new_plan = replacement_plan(&fixture);
    let new_assignment = fixture.engine.new_assignment_id().unwrap();
    let restored_b = std::cell::Cell::new(false);
    let prepared_a = std::cell::Cell::new(false);
    let error = fixture
        .engine
        .change_plugin(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            || Ok(()),
            || {
                prepared_a.set(true);
                Ok(())
            },
            &new_assignment,
            "2.0.0",
            &new_plan,
            &BTreeMap::new(),
            || Err("replacement patch refused".into()),
            || {
                restored_b.set(true);
                Ok(())
            },
            FailureInjection::default(),
        )
        .unwrap_err();
    assert!(error.old_plugin_reinstalled, "{error}");
    assert!(restored_b.get());
    assert!(prepared_a.get());
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-v1"
    );
    assert_eq!(
        fixture
            .engine
            .store()
            .load(1, &old_assignment)
            .unwrap()
            .state,
        AssignmentState::Active
    );
    assert_eq!(
        fixture
            .engine
            .store()
            .load(1, &new_assignment)
            .unwrap()
            .state,
        AssignmentState::Removed
    );
}

fn config_fixture(config_exists: bool) -> Fixture {
    let mut fixture = fixture(true);
    fs::write(fixture.package.join("config.template.ini"), b"template-v1").unwrap();
    if config_exists {
        fs::write(fixture.game.join("config.ini"), b"operator-original").unwrap();
    }
    fixture.manifest.config_files = vec![ConfigFile {
        config_file_location: "config.template.ini".into(),
        config_file_copy_type: crate::plugin::manifest::CopyType::Absolute,
        config_file_replace_path: Some("./config.ini".into()),
        config_add_to_config_menu: true,
    }];
    fixture.plan = build_plan(
        &fixture.package,
        &fixture.manifest,
        &"f".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();
    fixture
}

fn config_destination(fixture: &Fixture) -> PathBuf {
    fixture.game.join("config.ini")
}

fn rebuilt_config_plan(fixture: &Fixture) -> DeploymentPlan {
    build_plan(
        &fixture.package,
        &fixture.manifest,
        &"f".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap()
}

#[test]
fn an_edited_configuration_is_never_rewritten_by_a_later_deployment() {
    let fixture = config_fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);

    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"template-v1");

    fs::write(&destination, b"operator-tuned").unwrap();
    for pass in 0..3 {
        let plan = rebuilt_config_plan(&fixture);
        fixture
            .engine
            .deploy(
                &assignment,
                "1.0.0",
                &plan,
                &BTreeMap::new(),
                FailureInjection::default(),
            )
            .unwrap_or_else(|error| panic!("deployment pass {pass} was refused: {error}"));
        assert_eq!(
            fs::read(&destination).unwrap(),
            b"operator-tuned",
            "deployment pass {pass} rewrote the operator configuration"
        );
    }
}

#[test]
fn an_existing_configuration_survives_first_install_untouched() {
    let fixture = config_fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);

    let installed = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"operator-original");
    let record = installed
        .files
        .iter()
        .find(|file| paths_equal(&file.destination, &destination))
        .expect("the configuration destination must be recorded");
    assert!(
        record.backup_chain.is_empty(),
        "nothing was replaced, so nothing may be backed up"
    );

    let siblings: Vec<String> = fs::read_dir(&fixture.game)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("config.ini."))
        .collect();
    assert!(siblings.is_empty(), "unexpected backups: {siblings:?}");
}

#[test]
fn an_existing_configuration_is_replaced_only_when_permitted_at_initial_setup() {
    let fixture = config_fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    let decisions = BTreeMap::from([(
        decision_key(&destination),
        ModificationDecision::BackupAndOverwrite,
    )]);

    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &decisions,
            FailureInjection::default(),
        )
        .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"template-v1");
    let backups: Vec<PathBuf> = fs::read_dir(&fixture.game)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("config.ini.") && name.ends_with(".original"))
        })
        .collect();
    assert_eq!(
        backups.len(),
        1,
        "the replaced bytes must be backed up once"
    );
    assert_eq!(fs::read(&backups[0]).unwrap(), b"operator-original");
}

#[test]
fn the_same_permission_does_not_replace_a_configuration_after_initial_setup() {
    let fixture = config_fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(&destination, b"operator-tuned").unwrap();

    let decisions = BTreeMap::from([(
        decision_key(&destination),
        ModificationDecision::BackupAndOverwrite,
    )]);
    let plan = rebuilt_config_plan(&fixture);
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &plan,
            &decisions,
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"operator-tuned");
}

#[test]
fn a_modified_configuration_never_appears_as_a_preview_modification() {
    let fixture = config_fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    let ledger = fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(&destination, b"operator-tuned").unwrap();
    fs::write(fixture.game.join("plugin.bin"), b"operator-touched-binary").unwrap();

    let plan = rebuilt_config_plan(&fixture);
    let modifications = fixture
        .engine
        .deployment_modifications(Some(&ledger), &plan)
        .unwrap();

    assert!(
        modifications
            .iter()
            .all(|entry| !paths_equal(&entry.destination, &destination)),
        "a configuration file must never be offered as a modification decision"
    );
    assert!(
        modifications
            .iter()
            .any(|entry| paths_equal(&entry.destination, &fixture.game.join("plugin.bin"))),
        "a modified plugin binary must still be reported"
    );
}

#[test]
fn removing_an_assignment_leaves_the_configuration_in_place() {
    let fixture = config_fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(&destination, b"operator-tuned").unwrap();

    fixture
        .engine
        .remove(
            1,
            &assignment,
            &BTreeMap::new(),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"operator-tuned");
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"original"
    );
}

#[test]
fn changing_plugins_never_replaces_the_operator_configuration() {
    let fixture = config_fixture(false);
    let old_assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    fixture
        .engine
        .deploy(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(&destination, b"operator-tuned").unwrap();

    let package_b = fixture.package.with_file_name("package-config-b");
    fs::create_dir_all(&package_b).unwrap();
    fs::write(package_b.join("plugin.bin"), b"plugin-b").unwrap();
    fs::write(package_b.join("config.template.ini"), b"template-b").unwrap();
    let mut manifest_b = fixture.manifest.clone();
    manifest_b.plugin_id = "org.example.replacement".into();
    manifest_b.plugin_version = Version::parse("2.0.0").unwrap();
    fs::write(
        package_b.join("manifest.toml"),
        manifest_b.canonical_toml().unwrap(),
    )
    .unwrap();
    let new_plan = build_plan(
        &package_b,
        &manifest_b,
        &"9".repeat(64),
        1,
        &fixture.game.join("game.exe"),
        &ResolutionChoices::default(),
        &fixture.policy,
    )
    .unwrap();

    let new_assignment = fixture.engine.new_assignment_id().unwrap();
    let overwrite = BTreeMap::from([(
        decision_key(&destination),
        ModificationDecision::BackupAndOverwrite,
    )]);
    fixture
        .engine
        .change_plugin(
            &old_assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            || Ok(()),
            || Ok(()),
            &new_assignment,
            "2.0.0",
            &new_plan,
            &overwrite,
            || Ok(()),
            || Ok(()),
            FailureInjection::default(),
        )
        .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"operator-tuned");
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-b",
        "the replacement plugin's own files must still be deployed"
    );
}

#[test]
fn an_operator_reset_backs_up_then_re_applies_the_plugin_default() {
    let fixture = config_fixture(false);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    fs::write(&destination, b"operator-tuned").unwrap();

    let reset = fixture
        .engine
        .reset_configuration_files(1, &assignment)
        .unwrap();

    assert_eq!(reset.restored, vec![destination.clone()]);
    assert_eq!(reset.backups.len(), 1);
    assert_eq!(fs::read(&destination).unwrap(), b"template-v1");
    assert_eq!(fs::read(&reset.backups[0]).unwrap(), b"operator-tuned");

    let ledger = fixture.engine.store().load(1, &assignment).unwrap();
    let record = ledger
        .files
        .iter()
        .find(|file| paths_equal(&file.destination, &destination))
        .expect("the configuration destination must be recorded");
    assert!(
        record
            .backup_chain
            .iter()
            .any(|backup| backup.reason == "configuration-reset"),
        "the reset must be journaled in the backup chain"
    );
    assert_eq!(
        record.last_deployed_sha256,
        sha256_file(&destination).unwrap()
    );
}

#[test]
fn a_reset_is_refused_when_the_plugin_declares_no_configuration() {
    let fixture = fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();

    let error = fixture
        .engine
        .reset_configuration_files(1, &assignment)
        .expect_err("a plugin with no configuration files cannot be reset");
    assert!(
        error.contains("no configuration files"),
        "unexpected refusal: {error}"
    );
    assert_eq!(
        fs::read(fixture.game.join("plugin.bin")).unwrap(),
        b"plugin-v1",
        "a refused reset must not touch any plugin file"
    );
}

#[test]
fn a_reset_works_for_a_configuration_that_was_preserved_at_first_install() {
    let fixture = config_fixture(true);
    let assignment = fixture.engine.new_assignment_id().unwrap();
    let destination = config_destination(&fixture);
    fixture
        .engine
        .deploy(
            &assignment,
            "1.0.0",
            &fixture.plan,
            &BTreeMap::new(),
            FailureInjection::default(),
        )
        .unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"operator-original");

    let reset = fixture
        .engine
        .reset_configuration_files(1, &assignment)
        .unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"template-v1");
    assert_eq!(fs::read(&reset.backups[0]).unwrap(), b"operator-original");
}
