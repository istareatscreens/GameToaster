use std::fs;

use super::backup::{
    create_dated_backup, move_new, move_to_recovery, replace_existing, stage_bytes_adjacent,
};
use super::{
    AssignmentLedger, DeploymentEngine, FailureInjection, FailurePoint, FileCommitState,
    TransactionFileAction, TransactionState,
};
use crate::plugin::catalog::inventory::sha256_file;

impl DeploymentEngine {
    pub(super) fn execute_write_transaction(
        &self,
        ledger: &mut AssignmentLedger,
        index: usize,
        injection: FailureInjection,
    ) -> Result<(), String> {
        maybe_fail(injection.fail, FailurePoint::AfterPlanned)?;
        let transaction_id = ledger.transactions[index].transaction_id.clone();
        for file_index in 0..ledger.transactions[index].files.len() {
            if ledger.transactions[index].files[file_index].state == FileCommitState::KeptByUser {
                continue;
            }
            if ledger.transactions[index].files[file_index].action
                == TransactionFileAction::RemoveToRecovery
            {
                ledger.transactions[index].files[file_index].state = FileCommitState::Staged;
                self.store.save(ledger)?;
                maybe_fail(injection.fail, FailurePoint::AfterStaged(file_index))?;
                continue;
            }
            let source = ledger.transactions[index].files[file_index]
                .source
                .clone()
                .ok_or_else(|| "deployment transaction source is missing".to_string())?;
            let bytes = fs::read(&source)
                .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
            let expected = ledger.transactions[index].files[file_index]
                .expected_sha256
                .clone()
                .ok_or_else(|| "deployment expected hash is missing".to_string())?;
            if sha256_file(&source)? != expected {
                return Err(format!(
                    "deployment source changed after planning: {}",
                    source.display()
                ));
            }
            let destination = ledger.transactions[index].files[file_index]
                .destination
                .clone();
            let staged = stage_bytes_adjacent(
                &destination,
                &bytes,
                &format!("{transaction_id}-{file_index}"),
            )?;
            ledger.transactions[index].files[file_index].staged_path = Some(staged);
            ledger.transactions[index].files[file_index].state = FileCommitState::Staged;
            self.store.save(ledger)?;
            maybe_fail(injection.fail, FailurePoint::AfterStaged(file_index))?;
        }
        ledger.transactions[index].state = TransactionState::Staged;
        self.store.save(ledger)?;
        self.backup_transaction_files(ledger, index, injection.fail)?;
        self.commit_transaction_files(ledger, index, injection.fail)
    }

    pub(super) fn execute_removal_transaction(
        &self,
        ledger: &mut AssignmentLedger,
        index: usize,
        injection: FailureInjection,
    ) -> Result<(), String> {
        maybe_fail(injection.fail, FailurePoint::AfterPlanned)?;
        let transaction_id = ledger.transactions[index].transaction_id.clone();
        for file_index in 0..ledger.transactions[index].files.len() {
            let file = &ledger.transactions[index].files[file_index];
            if file.state == FileCommitState::KeptByUser {
                continue;
            }
            if let Some(source) = file.source.clone() {
                let bytes = fs::read(&source)
                    .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
                let destination = file.destination.clone();
                let staged = stage_bytes_adjacent(
                    &destination,
                    &bytes,
                    &format!("{transaction_id}-remove-{file_index}"),
                )?;
                ledger.transactions[index].files[file_index].staged_path = Some(staged);
            }
            ledger.transactions[index].files[file_index].state = FileCommitState::Staged;
            self.store.save(ledger)?;
            maybe_fail(injection.fail, FailurePoint::AfterStaged(file_index))?;
        }
        ledger.transactions[index].state = TransactionState::Staged;
        self.store.save(ledger)?;
        self.backup_transaction_files(ledger, index, injection.fail)?;

        ledger.transactions[index].state = TransactionState::Committing;
        self.store.save(ledger)?;
        maybe_fail(injection.fail, FailurePoint::AfterCommitting)?;
        for file_index in 0..ledger.transactions[index].files.len() {
            let state = ledger.transactions[index].files[file_index].state;
            if state == FileCommitState::KeptByUser {
                continue;
            }
            let destination = ledger.transactions[index].files[file_index]
                .destination
                .clone();
            if let Some(staged) = ledger.transactions[index].files[file_index]
                .staged_path
                .clone()
            {
                if destination.exists() {
                    replace_existing(&destination, &staged)?;
                } else {
                    move_new(&staged, &destination)?;
                }
                let expected = ledger.transactions[index].files[file_index]
                    .expected_sha256
                    .as_deref()
                    .ok_or_else(|| "removal restore hash is missing".to_string())?;
                if sha256_file(&destination)? != expected {
                    return Err(format!(
                        "restored file verification failed: {}",
                        destination.display()
                    ));
                }
            } else if destination.exists() {
                let recovery_root = self
                    .store
                    .assignment_root(ledger.game_id, &ledger.assignment_id)?
                    .join("recovery");
                let moved = move_to_recovery(
                    &destination,
                    &recovery_root,
                    &ledger.transactions[index].transaction_id,
                )?;
                ledger.transactions[index].files[file_index].recovery_path = Some(moved);
            }
            ledger.transactions[index].files[file_index].state = FileCommitState::Committed;
            self.store.save(ledger)?;
            maybe_fail(injection.fail, FailurePoint::AfterCommitted(file_index))?;
        }
        Ok(())
    }

    fn backup_transaction_files(
        &self,
        ledger: &mut AssignmentLedger,
        index: usize,
        failure: FailurePoint,
    ) -> Result<(), String> {
        for file_index in 0..ledger.transactions[index].files.len() {
            if ledger.transactions[index].files[file_index].state == FileCommitState::KeptByUser {
                continue;
            }
            let destination = ledger.transactions[index].files[file_index]
                .destination
                .clone();
            if destination.is_file() {
                let (backup, hash) = create_dated_backup(&destination)?;
                if ledger.transactions[index].files[file_index]
                    .pre_hash
                    .as_deref()
                    != Some(hash.as_str())
                {
                    return Err(format!(
                        "destination changed before backup: {}",
                        destination.display()
                    ));
                }
                ledger.transactions[index].files[file_index].transaction_backup = Some(backup);
            }
            ledger.transactions[index].files[file_index].state = FileCommitState::BackedUp;
            self.store.save(ledger)?;
            maybe_fail(failure, FailurePoint::AfterBackedUp(file_index))?;
        }
        ledger.transactions[index].state = TransactionState::BackedUp;
        self.store.save(ledger)
    }

    fn commit_transaction_files(
        &self,
        ledger: &mut AssignmentLedger,
        index: usize,
        failure: FailurePoint,
    ) -> Result<(), String> {
        ledger.transactions[index].state = TransactionState::Committing;
        self.store.save(ledger)?;
        maybe_fail(failure, FailurePoint::AfterCommitting)?;
        for file_index in 0..ledger.transactions[index].files.len() {
            if ledger.transactions[index].files[file_index].state == FileCommitState::KeptByUser {
                continue;
            }
            let destination = ledger.transactions[index].files[file_index]
                .destination
                .clone();
            if ledger.transactions[index].files[file_index].action
                == TransactionFileAction::RemoveToRecovery
            {
                if destination.exists() {
                    let recovery_root = self
                        .store
                        .assignment_root(ledger.game_id, &ledger.assignment_id)?
                        .join("recovery");
                    let moved = move_to_recovery(
                        &destination,
                        &recovery_root,
                        &ledger.transactions[index].transaction_id,
                    )?;
                    ledger.transactions[index].files[file_index].recovery_path = Some(moved);
                }
                ledger.transactions[index].files[file_index].state = FileCommitState::Committed;
                self.store.save(ledger)?;
                maybe_fail(failure, FailurePoint::AfterCommitted(file_index))?;
                continue;
            }
            let staged = ledger.transactions[index].files[file_index]
                .staged_path
                .clone()
                .ok_or_else(|| "transaction stage is missing".to_string())?;
            if ledger.transactions[index].files[file_index].preexisting {
                replace_existing(&destination, &staged)?;
            } else {
                move_new(&staged, &destination)?;
            }
            let expected = ledger.transactions[index].files[file_index]
                .expected_sha256
                .as_deref()
                .ok_or_else(|| "transaction expected hash is missing".to_string())?;
            if sha256_file(&destination)? != expected {
                return Err(format!(
                    "committed file verification failed: {}",
                    destination.display()
                ));
            }
            ledger.transactions[index].files[file_index].state = FileCommitState::Committed;
            self.store.save(ledger)?;
            maybe_fail(failure, FailurePoint::AfterCommitted(file_index))?;
        }
        ledger.transactions[index].state = TransactionState::Committed;
        self.store.save(ledger)
    }
}

fn maybe_fail(actual: FailurePoint, expected: FailurePoint) -> Result<(), String> {
    if actual == expected {
        Err(format!("injected failure at {expected:?}"))
    } else {
        Ok(())
    }
}
