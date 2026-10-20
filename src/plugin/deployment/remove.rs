use std::collections::BTreeMap;

use super::recovery::ensure_no_incomplete_transaction;
use super::{
    AssignmentLedger, AssignmentState, DeploymentEngine, FailureInjection, ModificationDecision,
    TransactionState,
};
use crate::support::fs::random_hex;

impl DeploymentEngine {
    pub fn remove<F>(
        &self,
        game_id: u64,
        assignment_id: &str,
        decisions: &BTreeMap<String, ModificationDecision>,
        restore_patcher: F,
        injection: FailureInjection,
    ) -> Result<AssignmentLedger, String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        let mut ledger = self.store.load(game_id, assignment_id)?;
        ensure_no_incomplete_transaction(&ledger)?;
        if ledger.state != AssignmentState::Active {
            return Err("only an active assignment can be removed".into());
        }

        restore_patcher().map_err(|error| {
            format!("plugin restore failed; no managed files were changed: {error}")
        })?;

        let transaction_id = random_hex(16)?;
        let transaction = self.plan_removal_transaction(&ledger, decisions, transaction_id)?;
        ledger.transactions.push(transaction);
        self.store.save(&ledger)?;
        let index = ledger.transactions.len() - 1;
        let result = self.execute_removal_transaction(&mut ledger, index, injection);
        match result {
            Ok(()) => {
                ledger.state = AssignmentState::Removed;
                ledger.transactions[index].state = TransactionState::Committed;
                self.store.save(&ledger)?;
                Ok(ledger)
            }
            Err(operation_error) => {
                let rollback = self.rollback_transaction(&mut ledger, index, injection.rollback_at);
                match rollback {
                    Ok(()) => Err(format!("{operation_error}; removal rolled back")),
                    Err(rollback_error) => Err(format!(
                        "{operation_error}; removal rollback requires recovery: {rollback_error}"
                    )),
                }
            }
        }
    }
}
