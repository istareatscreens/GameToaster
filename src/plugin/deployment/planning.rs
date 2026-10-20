use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::identity::hash_optional;
use super::{
    is_operator_owned, model, AssignmentLedger, DeploymentEngine, FileCommitState,
    FirstInstallOriginal, ModificationDecision, Transaction, TransactionFileAction,
    TransactionKind, TransactionState,
};
use crate::plugin::catalog::inventory::sha256_file;
use crate::plugin::paths::DeploymentPlan;
use crate::support::paths::{path_key, paths_equal};

impl DeploymentEngine {
    pub(super) fn plan_deployment_transaction(
        &self,
        ledger: &AssignmentLedger,
        plan: &DeploymentPlan,
        decisions: &BTreeMap<String, ModificationDecision>,
        transaction_id: String,
        kind: TransactionKind,
    ) -> Result<Transaction, String> {
        let mut files = Vec::with_capacity(plan.files.len());
        for planned in &plan.files {
            let current_hash = hash_optional(&planned.destination)?;
            if is_operator_owned(planned.kind) {
                let exists = current_hash.is_some();
                let permitted_at_initial_setup = kind == TransactionKind::Install
                    && decisions.get(&path_key(&planned.destination)).copied()
                        == Some(ModificationDecision::BackupAndOverwrite);
                let keep = exists && !permitted_at_initial_setup;
                files.push(model::TransactionFile {
                    action: if keep {
                        TransactionFileAction::KeepCurrent
                    } else {
                        TransactionFileAction::Write
                    },
                    destination: planned.destination.clone(),
                    source: Some(planned.source.clone()),
                    expected_sha256: Some(planned.source_sha256.clone()),
                    staged_path: None,
                    preexisting: exists,
                    pre_hash: current_hash,
                    transaction_backup: None,
                    recovery_path: None,
                    state: if keep {
                        FileCommitState::KeptByUser
                    } else {
                        FileCommitState::Planned
                    },
                });
                continue;
            }
            let prior = ledger
                .files
                .iter()
                .find(|file| paths_equal(&file.destination, &planned.destination));
            let modified = prior.is_some_and(|file| {
                current_hash.as_deref() != Some(file.last_deployed_sha256.as_str())
            });
            let decision = decisions.get(&path_key(&planned.destination)).copied();
            if modified && decision.is_none() {
                return Err(format!(
                    "managed file was modified by the user; explicit keep/replace decision required: {}",
                    planned.destination.display()
                ));
            }
            if decision == Some(ModificationDecision::RestoreOriginal) {
                return Err(format!(
                    "restore-original is a removal decision, not a deployment decision: {}",
                    planned.destination.display()
                ));
            }
            files.push(model::TransactionFile {
                action: if decision == Some(ModificationDecision::KeepCurrent) {
                    TransactionFileAction::KeepCurrent
                } else {
                    TransactionFileAction::Write
                },
                destination: planned.destination.clone(),
                source: Some(planned.source.clone()),
                expected_sha256: Some(planned.source_sha256.clone()),
                staged_path: None,
                preexisting: planned.destination.is_file(),
                pre_hash: current_hash,
                transaction_backup: None,
                recovery_path: None,
                state: if decision == Some(ModificationDecision::KeepCurrent) {
                    FileCommitState::KeptByUser
                } else {
                    FileCommitState::Planned
                },
            });
        }
        if kind == TransactionKind::Update {
            let desired: BTreeSet<_> = plan
                .files
                .iter()
                .map(|file| path_key(&file.destination))
                .collect();
            for managed in ledger
                .files
                .iter()
                .filter(|file| !desired.contains(&path_key(&file.destination)))
            {
                let current_hash = hash_optional(&managed.destination)?;
                if is_operator_owned(managed.kind) {
                    files.push(model::TransactionFile {
                        action: TransactionFileAction::KeepCurrent,
                        destination: managed.destination.clone(),
                        source: None,
                        expected_sha256: None,
                        staged_path: None,
                        preexisting: managed.destination.is_file(),
                        pre_hash: current_hash,
                        transaction_backup: None,
                        recovery_path: None,
                        state: FileCommitState::KeptByUser,
                    });
                    continue;
                }
                let modified =
                    current_hash.as_deref() != Some(managed.last_deployed_sha256.as_str());
                let decision = decisions.get(&path_key(&managed.destination)).copied();
                if modified && decision.is_none() {
                    return Err(format!(
                        "managed file removed by the new plugin version was modified by the user; explicit keep/restore decision required: {}",
                        managed.destination.display()
                    ));
                }
                if decision == Some(ModificationDecision::BackupAndOverwrite) {
                    return Err(format!(
                        "replace is not valid for a destination removed by the new plugin version: {}",
                        managed.destination.display()
                    ));
                }
                let keep = decision == Some(ModificationDecision::KeepCurrent);
                let (action, source, expected) = match &managed.first_install_original {
                    FirstInstallOriginal::Existing { backup, sha256 } => (
                        TransactionFileAction::RestoreOriginal,
                        Some(backup.clone()),
                        Some(sha256.clone()),
                    ),
                    FirstInstallOriginal::Absent => {
                        (TransactionFileAction::RemoveToRecovery, None, None)
                    }

                    FirstInstallOriginal::Preserved { .. } => {
                        (TransactionFileAction::KeepCurrent, None, None)
                    }
                };
                files.push(model::TransactionFile {
                    action: if keep {
                        TransactionFileAction::KeepCurrent
                    } else {
                        action
                    },
                    destination: managed.destination.clone(),
                    source,
                    expected_sha256: expected,
                    staged_path: None,
                    preexisting: managed.destination.is_file(),
                    pre_hash: current_hash,
                    transaction_backup: None,
                    recovery_path: None,
                    state: if keep {
                        FileCommitState::KeptByUser
                    } else {
                        FileCommitState::Planned
                    },
                });
            }
        }
        files.sort_by_key(|entry| path_key(&entry.destination));
        Ok(Transaction {
            transaction_id,
            kind,
            state: TransactionState::Planned,
            files,
        })
    }

    pub(super) fn plan_removal_transaction(
        &self,
        ledger: &AssignmentLedger,
        decisions: &BTreeMap<String, ModificationDecision>,
        transaction_id: String,
    ) -> Result<Transaction, String> {
        let mut files = Vec::with_capacity(ledger.files.len());
        for managed in &ledger.files {
            let current_hash = hash_optional(&managed.destination)?;
            if is_operator_owned(managed.kind) {
                files.push(model::TransactionFile {
                    action: TransactionFileAction::KeepCurrent,
                    destination: managed.destination.clone(),
                    source: None,
                    expected_sha256: None,
                    staged_path: None,
                    preexisting: managed.destination.is_file(),
                    pre_hash: current_hash,
                    transaction_backup: None,
                    recovery_path: None,
                    state: FileCommitState::KeptByUser,
                });
                continue;
            }
            let modified = current_hash.as_deref() != Some(managed.last_deployed_sha256.as_str());
            let decision = decisions.get(&path_key(&managed.destination)).copied();
            if modified && decision.is_none() {
                return Err(format!(
                    "managed file was modified by the user; explicit keep/restore decision required: {}",
                    managed.destination.display()
                ));
            }
            if decision == Some(ModificationDecision::BackupAndOverwrite) {
                return Err(format!(
                    "replace is not a valid removal decision; choose keep-current or restore-original: {}",
                    managed.destination.display()
                ));
            }
            let keep = decision == Some(ModificationDecision::KeepCurrent);
            let source = match &managed.first_install_original {
                FirstInstallOriginal::Existing { backup, sha256 } => {
                    if sha256_file(backup)? != *sha256 {
                        return Err(format!(
                            "first-install backup failed provenance validation: {}",
                            backup.display()
                        ));
                    }
                    Some(backup.clone())
                }
                FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => None,
            };
            files.push(model::TransactionFile {
                action: if keep {
                    TransactionFileAction::KeepCurrent
                } else if source.is_some() {
                    TransactionFileAction::RestoreOriginal
                } else {
                    TransactionFileAction::RemoveToRecovery
                },
                destination: managed.destination.clone(),
                source,
                expected_sha256: match &managed.first_install_original {
                    FirstInstallOriginal::Existing { sha256, .. } => Some(sha256.clone()),
                    FirstInstallOriginal::Absent | FirstInstallOriginal::Preserved { .. } => None,
                },
                staged_path: None,
                preexisting: managed.destination.is_file(),
                pre_hash: current_hash,
                transaction_backup: None,
                recovery_path: None,
                state: if keep {
                    FileCommitState::KeptByUser
                } else {
                    FileCommitState::Planned
                },
            });
        }
        Ok(Transaction {
            transaction_id,
            kind: TransactionKind::Remove,
            state: TransactionState::Planned,
            files,
        })
    }
}
