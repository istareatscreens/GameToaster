use std::collections::BTreeSet;

use super::identity::hash_optional;
use super::{
    is_operator_owned, model, AssignmentLedger, DeploymentEngine, ModificationPurpose,
    UserModification,
};
use crate::plugin::paths::DeploymentPlan;
use crate::support::paths::{path_key, paths_equal};

impl DeploymentEngine {
    pub fn deployment_modifications(
        &self,
        ledger: Option<&AssignmentLedger>,
        plan: &DeploymentPlan,
    ) -> Result<Vec<UserModification>, String> {
        let Some(ledger) = ledger else {
            return Ok(Vec::new());
        };
        let desired: BTreeSet<_> = plan
            .files
            .iter()
            .map(|file| path_key(&file.destination))
            .collect();
        let mut modifications = Vec::new();
        for planned in &plan.files {
            if is_operator_owned(planned.kind) {
                continue;
            }
            let Some(prior) = ledger
                .files
                .iter()
                .find(|file| paths_equal(&file.destination, &planned.destination))
            else {
                continue;
            };
            let current_sha256 = hash_optional(&prior.destination)?;
            if ledger.keeps_instead_of_writing(planned, current_sha256.as_deref()) {
                modifications.push(modification(
                    prior,
                    ModificationPurpose::ReplaceWithPlugin,
                    current_sha256,
                ));
            }
        }
        for retired in ledger
            .files
            .iter()
            .filter(|file| !desired.contains(&path_key(&file.destination)))
            .filter(|file| !is_operator_owned(file.kind))
        {
            push_if_left_as_is(&mut modifications, ledger, retired)?;
        }
        modifications.sort_by_key(|entry| path_key(&entry.destination));
        Ok(modifications)
    }

    pub fn removal_modifications(
        &self,
        ledger: &AssignmentLedger,
    ) -> Result<Vec<UserModification>, String> {
        let mut modifications = Vec::new();
        for file in &ledger.files {
            if is_operator_owned(file.kind) {
                continue;
            }
            push_if_left_as_is(&mut modifications, ledger, file)?;
        }
        modifications.sort_by_key(|entry| path_key(&entry.destination));
        Ok(modifications)
    }
}

fn push_if_left_as_is(
    output: &mut Vec<UserModification>,
    ledger: &AssignmentLedger,
    managed: &model::FileProvenance,
) -> Result<(), String> {
    let current_sha256 = hash_optional(&managed.destination)?;
    if ledger.leaves_instead_of_restoring(managed, current_sha256.as_deref()) {
        output.push(modification(
            managed,
            ModificationPurpose::RestoreOriginal,
            current_sha256,
        ));
    }
    Ok(())
}

fn modification(
    managed: &model::FileProvenance,
    purpose: ModificationPurpose,
    current_sha256: Option<String>,
) -> UserModification {
    UserModification {
        destination: managed.destination.clone(),
        purpose,
        last_deployed_sha256: managed.last_deployed_sha256.clone(),
        current_sha256,
    }
}
