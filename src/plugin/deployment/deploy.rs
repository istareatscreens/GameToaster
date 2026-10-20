use std::collections::BTreeMap;

use super::provenance::apply_committed_plan;
use super::recovery::ensure_no_incomplete_transaction;
use super::{
    AssignmentLedger, AssignmentState, DeploymentEngine, FailureInjection, ModificationDecision,
    TransactionKind,
};
use crate::plugin::paths::{revalidate_plan, DeploymentPlan};
use crate::support::fs::random_hex;

impl DeploymentEngine {
    pub fn deploy(
        &self,
        assignment_id: &str,
        plugin_version: &str,
        plan: &DeploymentPlan,
        decisions: &BTreeMap<String, ModificationDecision>,
        injection: FailureInjection,
    ) -> Result<AssignmentLedger, String> {
        revalidate_plan(plan, &self.path_policy)?;
        let mut ledger = match self.store.load_if_present(plan.game_id, assignment_id)? {
            Some(ledger) => {
                if ledger.state == AssignmentState::RecoveryRequired {
                    return Err("assignment requires recovery before deployment".into());
                }
                if ledger.plugin_id != plan.plugin_id
                    || ledger.game_folder != plan.game_folder
                    || ledger.selected_target != plan.selected_target
                {
                    return Err("deployment plan does not match the existing assignment".into());
                }
                ensure_no_incomplete_transaction(&ledger)?;
                ledger
            }
            None => {
                AssignmentLedger::new(assignment_id.to_string(), plugin_version.to_string(), plan)
            }
        };
        let kind = if ledger.files.is_empty() {
            TransactionKind::Install
        } else {
            TransactionKind::Update
        };
        let transaction_id = random_hex(16)?;
        let transaction =
            self.plan_deployment_transaction(&ledger, plan, decisions, transaction_id, kind)?;
        ledger.transactions.push(transaction);
        self.store.save(&ledger)?;
        let transaction_index = ledger.transactions.len() - 1;
        let result = self.execute_write_transaction(&mut ledger, transaction_index, injection);
        match result {
            Ok(()) => {
                apply_committed_plan(&mut ledger, transaction_index, plan, plugin_version)?;
                self.store.save(&ledger)?;
                Ok(ledger)
            }
            Err(operation_error) => {
                let rollback = self.rollback_transaction(
                    &mut ledger,
                    transaction_index,
                    injection.rollback_at,
                );
                match rollback {
                    Ok(()) => Err(format!("{operation_error}; transaction rolled back")),
                    Err(rollback_error) => Err(format!(
                        "{operation_error}; rollback requires recovery: {rollback_error}"
                    )),
                }
            }
        }
    }
}
