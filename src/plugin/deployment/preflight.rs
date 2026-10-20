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
            push_modification_if_changed(
                &mut modifications,
                prior,
                ModificationPurpose::ReplaceWithPlugin,
            )?;
        }
        for retired in ledger
            .files
            .iter()
            .filter(|file| !desired.contains(&path_key(&file.destination)))
            .filter(|file| !is_operator_owned(file.kind))
        {
            push_modification_if_changed(
                &mut modifications,
                retired,
                ModificationPurpose::RestoreOriginal,
            )?;
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
            push_modification_if_changed(
                &mut modifications,
                file,
                ModificationPurpose::RestoreOriginal,
            )?;
        }
        modifications.sort_by_key(|entry| path_key(&entry.destination));
        Ok(modifications)
    }
}

fn push_modification_if_changed(
    output: &mut Vec<UserModification>,
    managed: &model::FileProvenance,
    purpose: ModificationPurpose,
) -> Result<(), String> {
    let current_sha256 = hash_optional(&managed.destination)?;
    if current_sha256.as_deref() != Some(managed.last_deployed_sha256.as_str()) {
        output.push(UserModification {
            destination: managed.destination.clone(),
            purpose,
            last_deployed_sha256: managed.last_deployed_sha256.clone(),
            current_sha256,
        });
    }
    Ok(())
}
