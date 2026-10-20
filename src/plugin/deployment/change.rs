use std::collections::BTreeMap;

use super::{
    decision_key, is_operator_owned, AssignmentState, DeploymentEngine, FailureInjection,
    ModificationDecision, PluginChangeError, PluginChangeResult, TransactionKind,
};
use crate::plugin::paths::{rebind_destination_leaves, revalidate_plan, DeploymentPlan};
use crate::support::paths::path_key;

impl DeploymentEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn change_plugin<RestoreA, PrepareA, PrepareB, RestoreB>(
        &self,
        old_assignment_id: &str,
        old_plugin_version: &str,
        old_plan: &DeploymentPlan,
        old_decisions: &BTreeMap<String, ModificationDecision>,
        restore_old_patcher: RestoreA,
        prepare_old_after_reinstall: PrepareA,
        new_assignment_id: &str,
        new_plugin_version: &str,
        new_plan: &DeploymentPlan,
        new_decisions: &BTreeMap<String, ModificationDecision>,
        prepare_new_patcher: PrepareB,
        restore_new_after_failed_prepare: RestoreB,
        new_injection: FailureInjection,
    ) -> Result<PluginChangeResult, PluginChangeError>
    where
        RestoreA: FnOnce() -> Result<(), String>,
        PrepareA: FnOnce() -> Result<(), String>,
        PrepareB: FnOnce() -> Result<(), String>,
        RestoreB: FnOnce() -> Result<(), String>,
    {
        let prevalidate: Result<(), String> = (|| {
            if old_plan.game_id != new_plan.game_id
                || old_plan.game_folder != new_plan.game_folder
                || old_plan.selected_target != new_plan.selected_target
            {
                return Err("replacement plugin context does not match the existing game".into());
            }
            if old_assignment_id == new_assignment_id {
                return Err("replacement plugin requires a new assignment ID".into());
            }
            revalidate_plan(new_plan, &self.path_policy)?;
            if self
                .store
                .load_if_present(new_plan.game_id, new_assignment_id)?
                .is_some()
            {
                return Err("replacement assignment ID already exists".into());
            }
            let old = self.store.load(old_plan.game_id, old_assignment_id)?;
            if old.state != AssignmentState::Active
                || old.plugin_id != old_plan.plugin_id
                || old.revision_sha256 != old_plan.revision_sha256
            {
                return Err(
                    "previous assignment does not match its retained plugin version".into(),
                );
            }
            Ok(())
        })();
        if let Err(error) = prevalidate {
            return Err(PluginChangeError {
                new_plugin_error: format!("replacement prevalidation failed: {error}"),
                old_plugin_reinstalled: true,
                recovery_error: None,
            });
        }

        let removed = self
            .remove(
                old_plan.game_id,
                old_assignment_id,
                old_decisions,
                restore_old_patcher,
                FailureInjection::default(),
            )
            .map_err(|error| PluginChangeError {
                new_plugin_error: format!("previous plugin restore failed: {error}"),
                old_plugin_reinstalled: true,
                recovery_error: None,
            })?;

        let rebound_new = match rebind_destination_leaves(new_plan, &self.path_policy) {
            Ok(plan) => plan,
            Err(new_plugin_error) => {
                let recovery = self.restore_removed_assignment(
                    old_assignment_id,
                    old_plugin_version,
                    old_plan,
                    prepare_old_after_reinstall,
                );
                return Err(match recovery {
                    Ok(()) => PluginChangeError {
                        new_plugin_error,
                        old_plugin_reinstalled: true,
                        recovery_error: None,
                    },
                    Err(recovery_error) => PluginChangeError {
                        new_plugin_error,
                        old_plugin_reinstalled: false,
                        recovery_error: Some(recovery_error),
                    },
                });
            }
        };

        let new_decisions = &without_configuration_overwrites(&rebound_new, new_decisions);
        match self.deploy(
            new_assignment_id,
            new_plugin_version,
            &rebound_new,
            new_decisions,
            new_injection,
        ) {
            Ok(mut active) => {
                if let Err(prepare_error) = prepare_new_patcher() {
                    let removal_decisions = active
                        .files
                        .iter()
                        .map(|file| {
                            (
                                decision_key(&file.destination),
                                ModificationDecision::RestoreOriginal,
                            )
                        })
                        .collect();
                    if let Err(remove_error) = self.remove(
                        new_plan.game_id,
                        new_assignment_id,
                        &removal_decisions,
                        restore_new_after_failed_prepare,
                        FailureInjection::default(),
                    ) {
                        return Err(PluginChangeError {
                            new_plugin_error: format!(
                                "replacement prepare failed: {prepare_error}"
                            ),
                            old_plugin_reinstalled: false,
                            recovery_error: Some(format!(
                                "replacement restore/removal failed: {remove_error}"
                            )),
                        });
                    }
                    let recovery = self.restore_removed_assignment(
                        old_assignment_id,
                        old_plugin_version,
                        old_plan,
                        prepare_old_after_reinstall,
                    );
                    return Err(match recovery {
                        Ok(()) => PluginChangeError {
                            new_plugin_error: format!(
                                "replacement prepare failed: {prepare_error}"
                            ),
                            old_plugin_reinstalled: true,
                            recovery_error: None,
                        },
                        Err(recovery_error) => PluginChangeError {
                            new_plugin_error: format!(
                                "replacement prepare failed: {prepare_error}"
                            ),
                            old_plugin_reinstalled: false,
                            recovery_error: Some(recovery_error),
                        },
                    });
                }
                if let Some(transaction) = active.transactions.last_mut() {
                    transaction.kind = TransactionKind::Change;
                }
                if let Err(error) = self.store.save(&active) {
                    return Err(PluginChangeError {
                        new_plugin_error: format!(
                            "replacement installed but its change journal could not be finalized: {error}"
                        ),
                        old_plugin_reinstalled: false,
                        recovery_error: Some(
                            "do not launch; run assignment recovery before another operation".into(),
                        ),
                    });
                }
                Ok(PluginChangeResult {
                    removed_assignment: removed,
                    active_assignment: active,
                })
            }
            Err(new_plugin_error) => {
                let recovery = self.restore_removed_assignment(
                    old_assignment_id,
                    old_plugin_version,
                    old_plan,
                    prepare_old_after_reinstall,
                );
                match recovery {
                    Ok(()) => Err(PluginChangeError {
                        new_plugin_error,
                        old_plugin_reinstalled: true,
                        recovery_error: None,
                    }),
                    Err(recovery_error) => Err(PluginChangeError {
                        new_plugin_error,
                        old_plugin_reinstalled: false,
                        recovery_error: Some(recovery_error),
                    }),
                }
            }
        }
    }

    pub fn restore_removed_assignment<PrepareA>(
        &self,
        assignment_id: &str,
        plugin_version: &str,
        plan: &DeploymentPlan,
        prepare: PrepareA,
    ) -> Result<(), String>
    where
        PrepareA: FnOnce() -> Result<(), String>,
    {
        let rebound = rebind_destination_leaves(plan, &self.path_policy)?;
        let decisions = rebound
            .files
            .iter()
            .map(|file| {
                (
                    decision_key(&file.destination),
                    ModificationDecision::BackupAndOverwrite,
                )
            })
            .collect();
        self.deploy(
            assignment_id,
            plugin_version,
            &rebound,
            &decisions,
            FailureInjection::default(),
        )?;
        prepare()
    }
}

fn without_configuration_overwrites(
    plan: &DeploymentPlan,
    decisions: &BTreeMap<String, ModificationDecision>,
) -> BTreeMap<String, ModificationDecision> {
    let mut sanitized = decisions.clone();
    for planned in plan
        .files
        .iter()
        .filter(|file| is_operator_owned(file.kind))
    {
        if sanitized.get(&path_key(&planned.destination)).copied()
            == Some(ModificationDecision::BackupAndOverwrite)
        {
            sanitized.remove(&path_key(&planned.destination));
        }
    }
    sanitized
}
