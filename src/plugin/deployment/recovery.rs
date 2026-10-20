use std::fs;

use super::backup::{move_new, move_to_recovery, replace_existing, stage_bytes_adjacent};
use super::identity::hash_optional;
use super::{
    AssignmentLedger, AssignmentState, DeploymentEngine, FileCommitState, TransactionKind,
    TransactionState,
};

impl DeploymentEngine {
    pub fn recover_all(&self) -> Result<Vec<AssignmentLedger>, String> {
        let mut recovered = Vec::new();
        for mut ledger in self.store.list()? {
            let Some(index) = ledger
                .transactions
                .iter()
                .rposition(|transaction| !is_terminal(transaction.state))
            else {
                continue;
            };
            self.rollback_transaction(&mut ledger, index, None)?;
            recovered.push(ledger);
        }
        Ok(recovered)
    }

    pub(super) fn rollback_transaction(
        &self,
        ledger: &mut AssignmentLedger,
        index: usize,
        fail_at: Option<usize>,
    ) -> Result<(), String> {
        ledger.transactions[index].state = TransactionState::RollingBack;
        self.store.save(ledger)?;
        let mut errors = Vec::new();
        let length = ledger.transactions[index].files.len();
        for reverse_index in 0..length {
            let file_index = length - 1 - reverse_index;
            if fail_at == Some(reverse_index) {
                errors.push(format!("injected rollback failure at {reverse_index}"));
                break;
            }
            let state = ledger.transactions[index].files[file_index].state;
            if state == FileCommitState::KeptByUser || state == FileCommitState::RolledBack {
                continue;
            }
            let result = self.rollback_file(ledger, index, file_index);
            match result {
                Ok(()) => {
                    ledger.transactions[index].files[file_index].state =
                        FileCommitState::RolledBack;
                    if let Err(error) = self.store.save(ledger) {
                        errors.push(error);
                        break;
                    }
                }
                Err(error) => {
                    ledger.transactions[index].files[file_index].state =
                        FileCommitState::RecoveryRequired;
                    errors.push(error);
                    let _ = self.store.save(ledger);
                    break;
                }
            }
        }
        if errors.is_empty() {
            ledger.transactions[index].state = TransactionState::RolledBack;
            ledger.state = if ledger.transactions[index].kind == TransactionKind::Install
                && ledger.files.is_empty()
            {
                AssignmentState::Removed
            } else {
                AssignmentState::Active
            };
            self.store.save(ledger)
        } else {
            ledger.transactions[index].state = TransactionState::RecoveryRequired;
            ledger.state = AssignmentState::RecoveryRequired;
            self.store.save(ledger)?;
            Err(errors.join("; "))
        }
    }

    fn rollback_file(
        &self,
        ledger: &mut AssignmentLedger,
        transaction_index: usize,
        file_index: usize,
    ) -> Result<(), String> {
        let file = ledger.transactions[transaction_index].files[file_index].clone();
        if file.state == FileCommitState::Committed {
            if file.preexisting {
                let backup = file.transaction_backup.ok_or_else(|| {
                    format!(
                        "rollback backup is missing for {}",
                        file.destination.display()
                    )
                })?;
                let bytes = fs::read(&backup)
                    .map_err(|error| format!("failed to read {}: {error}", backup.display()))?;
                let staged = stage_bytes_adjacent(
                    &file.destination,
                    &bytes,
                    &format!(
                        "{}-rollback-{file_index}",
                        ledger.transactions[transaction_index].transaction_id
                    ),
                )?;
                if file.destination.exists() {
                    replace_existing(&file.destination, &staged)?;
                } else {
                    move_new(&staged, &file.destination)?;
                }
                if hash_optional(&file.destination)? != file.pre_hash {
                    return Err(format!(
                        "rollback verification failed: {}",
                        file.destination.display()
                    ));
                }
            } else if file.destination.exists() {
                let recovery_root = self
                    .store
                    .assignment_root(ledger.game_id, &ledger.assignment_id)?
                    .join("recovery");
                let moved = move_to_recovery(
                    &file.destination,
                    &recovery_root,
                    &format!(
                        "{}-rollback",
                        ledger.transactions[transaction_index].transaction_id
                    ),
                )?;
                ledger.transactions[transaction_index].files[file_index].recovery_path =
                    Some(moved);
            }
        } else if let Some(staged) = file.staged_path {
            if staged.exists() {
                let recovery_root = self
                    .store
                    .assignment_root(ledger.game_id, &ledger.assignment_id)?
                    .join("recovery");
                let moved = move_to_recovery(
                    &staged,
                    &recovery_root,
                    &format!(
                        "{}-stage",
                        ledger.transactions[transaction_index].transaction_id
                    ),
                )?;
                ledger.transactions[transaction_index].files[file_index].recovery_path =
                    Some(moved);
            }
        }
        Ok(())
    }
}

pub(super) fn ensure_no_incomplete_transaction(ledger: &AssignmentLedger) -> Result<(), String> {
    if ledger
        .transactions
        .iter()
        .any(|transaction| !is_terminal(transaction.state))
    {
        Err("assignment has an incomplete transaction; run recovery first".into())
    } else {
        Ok(())
    }
}

fn is_terminal(state: TransactionState) -> bool {
    matches!(
        state,
        TransactionState::Committed | TransactionState::RolledBack
    )
}
