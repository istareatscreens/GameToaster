use std::collections::BTreeSet;

use super::{
    model, AssignmentLedger, AssignmentState, BackupRecord, FileCommitState, FirstInstallOriginal,
    TransactionKind, TransactionState,
};
use crate::plugin::catalog::inventory::sha256_file;
use crate::plugin::paths::DeploymentPlan;
use crate::support::paths::{path_key, paths_equal};

pub(super) fn apply_committed_plan(
    ledger: &mut AssignmentLedger,
    transaction_index: usize,
    plan: &DeploymentPlan,
    plugin_version: &str,
) -> Result<(), String> {
    let desired: BTreeSet<_> = plan
        .files
        .iter()
        .map(|file| path_key(&file.destination))
        .collect();
    for planned in &plan.files {
        if !ledger
            .files
            .iter()
            .any(|file| paths_equal(&file.destination, &planned.destination))
        {
            if let Some(index) = ledger
                .retired_files
                .iter()
                .position(|file| paths_equal(&file.destination, &planned.destination))
            {
                ledger.files.push(ledger.retired_files.remove(index));
            }
        }
        let transaction_file = ledger.transactions[transaction_index]
            .files
            .iter()
            .find(|file| paths_equal(&file.destination, &planned.destination))
            .ok_or_else(|| {
                format!(
                    "transaction is missing planned destination {}",
                    planned.destination.display()
                )
            })?;
        if transaction_file.state == FileCommitState::KeptByUser {
            let current = sha256_file(&planned.destination)?;
            if let Some(existing) = ledger
                .files
                .iter_mut()
                .find(|file| paths_equal(&file.destination, &planned.destination))
            {
                existing.last_deployed_sha256 = current;
                existing.user_kept_current = true;
            } else {
                ledger.files.push(model::FileProvenance {
                    kind: planned.kind,
                    source_relative: planned.source_relative.clone(),
                    source: planned.source.clone(),
                    destination: planned.destination.clone(),
                    first_install_original: FirstInstallOriginal::Preserved {
                        sha256: current.clone(),
                    },
                    backup_chain: Vec::new(),
                    last_deployed_sha256: current,
                    add_to_config_menu: planned.add_to_config_menu,
                    user_kept_current: true,
                });
            }
            continue;
        }
        let backup_record = transaction_file
            .transaction_backup
            .as_ref()
            .zip(transaction_file.pre_hash.as_ref())
            .map(|(path, hash)| BackupRecord {
                path: path.clone(),
                sha256: hash.clone(),
                reason: match ledger.transactions[transaction_index].kind {
                    TransactionKind::Install => "first-install".into(),
                    _ => "update".into(),
                },
            });
        if let Some(existing) = ledger
            .files
            .iter_mut()
            .find(|file| paths_equal(&file.destination, &planned.destination))
        {
            if let Some(backup) = backup_record {
                existing.backup_chain.push(backup);
            }
            existing.kind = planned.kind;
            existing.source_relative = planned.source_relative.clone();
            existing.source = planned.source.clone();
            existing.last_deployed_sha256 = planned.source_sha256.clone();
            existing.add_to_config_menu = planned.add_to_config_menu;
            existing.user_kept_current = false;
        } else {
            let first_install_original = match &backup_record {
                Some(backup) => FirstInstallOriginal::Existing {
                    backup: backup.path.clone(),
                    sha256: backup.sha256.clone(),
                },
                None => FirstInstallOriginal::Absent,
            };
            ledger.files.push(model::FileProvenance {
                kind: planned.kind,
                source_relative: planned.source_relative.clone(),
                source: planned.source.clone(),
                destination: planned.destination.clone(),
                first_install_original,
                backup_chain: backup_record.into_iter().collect(),
                last_deployed_sha256: planned.source_sha256.clone(),
                add_to_config_menu: planned.add_to_config_menu,
                user_kept_current: false,
            });
        }
    }
    let previous = std::mem::take(&mut ledger.files);
    for mut managed in previous {
        if desired.contains(&path_key(&managed.destination)) {
            ledger.files.push(managed);
            continue;
        }
        if let Some(transaction_file) = ledger.transactions[transaction_index]
            .files
            .iter()
            .find(|file| paths_equal(&file.destination, &managed.destination))
        {
            if let Some((backup, hash)) = transaction_file
                .transaction_backup
                .as_ref()
                .zip(transaction_file.pre_hash.as_ref())
            {
                managed.backup_chain.push(BackupRecord {
                    path: backup.clone(),
                    sha256: hash.clone(),
                    reason: "removed-by-update".into(),
                });
            }
        }
        ledger.retired_files.push(managed);
    }
    ledger
        .files
        .sort_by_key(|entry| path_key(&entry.destination));
    ledger
        .retired_files
        .sort_by_key(|entry| path_key(&entry.destination));
    ledger.plugin_version = plugin_version.to_string();
    ledger.revision_sha256 = plan.revision_sha256.clone();
    ledger.resolved_launcher = plan.resolved_launcher.clone();
    ledger.resolved_watch_target = plan.resolved_watch_target.clone();
    ledger.state = AssignmentState::Active;
    ledger.transactions[transaction_index].state = TransactionState::Committed;
    Ok(())
}
