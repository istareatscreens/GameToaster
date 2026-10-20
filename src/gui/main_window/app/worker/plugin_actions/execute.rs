use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc;

use eframe::egui;
use rust_i18n::t;

use super::change::execute_approved_change;
use super::removal::execute_approved_removal;
use super::update::execute_approved_update;
use super::{PendingPluginAction, PendingPluginKind};
use crate::gui::main_window::app::worker::messages::WorkerResult;
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::launcher_gui::config;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::{
    GameEntry, GameId, LauncherConfig, PluginAssignment, PluginRevision, PluginUpdatePolicy,
};
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::deployment::ModificationDecision;
use crate::plugin::workflow::PluginActionApproval;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_pending_plugin_action(
    config_path: &Path,
    mut pending: PendingPluginAction,
    approval: PluginActionApproval,
    backend: &Result<PluginBackend, String>,
    session_root: &Path,
    game_state: &crate::gui::common::game_state::GameStateHub,
    results: &mpsc::Sender<WorkerResult>,
    ctx: &egui::Context,
    log: &LauncherLog,
) -> Result<LauncherConfig, String> {
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    backend.deployment.recover_all()?;

    let approved = pending
        .preview
        .take()
        .map(|preview| {
            preview.approve(
                &approval.granted_external_roots,
                approval.launcher_destination.as_deref(),
                approval.watch_destination.as_deref(),
            )
        })
        .transpose()?;
    let (plan, new_grants) = match approved {
        Some((plan, grants)) => (Some(plan), grants),
        None => (None, Vec::new()),
    };
    for grant in new_grants {
        if !pending
            .choices
            .external_grants
            .iter()
            .any(|existing| existing == &grant)
        {
            pending.choices.external_grants.push(grant);
        }
    }

    let modifications = match (
        &pending.kind,
        plan.as_ref(),
        pending.existing_ledger.as_ref(),
    ) {
        (PendingPluginKind::Remove { .. } | PendingPluginKind::Change { .. }, _, Some(ledger)) => {
            backend.deployment.removal_modifications(ledger)?
        }
        (_, Some(plan), ledger) => backend.deployment.deployment_modifications(ledger, plan)?,
        _ => Vec::new(),
    };
    validate_modification_approval(&modifications, &approval.modification_decisions)?;

    match pending.kind {
        PendingPluginKind::Add {
            mut config,
            mut game,
            selection,
            assignment_id,
        } => {
            if config.game(game.id).is_some() {
                return Err(t!("errors.game_exists", id => game.id).to_string());
            }
            let plan = plan.ok_or_else(|| t!("plugins.error.plan_missing_add").to_string())?;
            let mut ledger = backend.deployment.deploy(
                &assignment_id,
                &pending.revision.plugin_version.to_string(),
                &plan,
                &approval.modification_decisions,
                crate::plugin::deployment::FailureInjection::default(),
            )?;
            if let Err(error) = crate::plugin::lifecycle::install_scripts::run_pending(
                backend.deployment.store(),
                &pending.revision.root,
                &pending.manifest,
                &mut ledger,
            ) {
                return Err(rollback_new_assignment_after_config_error(
                    backend,
                    game.id,
                    &assignment_id,
                    &ledger,
                    error,
                ));
            }
            apply_new_assignment(
                &mut game,
                &pending.revision,
                &selection,
                assignment_id.clone(),
                &ledger,
                pending.choices.external_grants,
                backend,
            )?;
            config.games.push(game.clone());
            if let Err(error) = config::save_atomic(config_path, &config) {
                return Err(rollback_new_assignment_after_config_error(
                    backend,
                    game.id,
                    &assignment_id,
                    &ledger,
                    error,
                ));
            }
            Ok(config)
        }
        PendingPluginKind::Install {
            mut config,
            game_index,
            selection,
            assignment_id,
        } => {
            let plan = plan.ok_or_else(|| t!("plugins.error.plan_missing_install").to_string())?;
            let game_id = config
                .games
                .get(game_index)
                .ok_or_else(|| t!("plugins.error.game_gone_before_install").to_string())?
                .id;
            if config.games[game_index].plugin.is_some() {
                return Err(t!("plugins.error.assignment_changed").to_string());
            }
            let mut ledger = backend.deployment.deploy(
                &assignment_id,
                &pending.revision.plugin_version.to_string(),
                &plan,
                &approval.modification_decisions,
                crate::plugin::deployment::FailureInjection::default(),
            )?;
            if let Err(error) = crate::plugin::lifecycle::install_scripts::run_pending(
                backend.deployment.store(),
                &pending.revision.root,
                &pending.manifest,
                &mut ledger,
            ) {
                return Err(rollback_new_assignment_after_config_error(
                    backend,
                    game_id,
                    &assignment_id,
                    &ledger,
                    error,
                ));
            }
            apply_new_assignment(
                &mut config.games[game_index],
                &pending.revision,
                &selection,
                assignment_id.clone(),
                &ledger,
                pending.choices.external_grants,
                backend,
            )?;
            if let Err(error) = config::save_atomic(config_path, &config) {
                return Err(rollback_new_assignment_after_config_error(
                    backend,
                    game_id,
                    &assignment_id,
                    &ledger,
                    error,
                ));
            }
            Ok(config)
        }
        PendingPluginKind::Update {
            config,
            game_index,
            selection,
        }
        | PendingPluginKind::Repair {
            config,
            game_index,
            selection,
        } => execute_approved_update(
            config_path,
            config,
            game_index,
            selection,
            &pending.revision,
            &pending.manifest,
            plan.ok_or_else(|| t!("plugins.error.plan_missing_update").to_string())?,
            pending.choices.external_grants,
            &approval.modification_decisions,
            backend,
            session_root,
            game_state,
            results,
            ctx,
            log,
        ),
        PendingPluginKind::Change {
            config,
            game_index,
            selection,
            old_plan,
            old_manifest,
            new_assignment_id,
        } => execute_approved_change(
            config_path,
            config,
            game_index,
            selection,
            &pending.revision,
            &pending.manifest,
            plan.ok_or_else(|| t!("plugins.error.plan_missing_replacement").to_string())?,
            pending.choices.external_grants,
            old_plan,
            old_manifest,
            new_assignment_id,
            &approval.modification_decisions,
            backend,
            session_root,
            game_state,
            results,
            ctx,
            log,
        ),
        PendingPluginKind::Remove { config, game_index } => execute_approved_removal(
            config_path,
            config,
            game_index,
            &pending.revision,
            &pending.manifest,
            &approval.modification_decisions,
            backend,
            session_root,
            game_state,
            results,
            ctx,
            log,
        ),
    }
}

pub(super) fn validate_modification_approval(
    modifications: &[crate::plugin::deployment::UserModification],
    decisions: &BTreeMap<String, ModificationDecision>,
) -> Result<(), String> {
    use crate::plugin::deployment::ModificationPurpose;
    for modification in modifications {
        let key = crate::plugin::deployment::decision_key(&modification.destination);
        let decision = decisions.get(&key).copied().ok_or_else(|| {
            t!(
                "plugins.error.modification_unapproved",
                path => modification.destination.display().to_string()
            )
            .to_string()
        })?;
        let valid = match modification.purpose {
            ModificationPurpose::ReplaceWithPlugin => matches!(
                decision,
                ModificationDecision::BackupAndOverwrite | ModificationDecision::KeepCurrent
            ),
            ModificationPurpose::RestoreOriginal => matches!(
                decision,
                ModificationDecision::RestoreOriginal | ModificationDecision::KeepCurrent
            ),
        };
        if !valid {
            return Err(t!(
                "plugins.error.modification_invalid",
                path => modification.destination.display().to_string()
            )
            .to_string());
        }
    }
    Ok(())
}

pub(super) fn apply_new_assignment(
    game: &mut GameEntry,
    revision: &crate::plugin::catalog::CatalogRevision,
    selection: &PluginSelection,
    assignment_id: String,
    ledger: &crate::plugin::deployment::AssignmentLedger,
    grants: Vec<crate::plugin::paths::ExternalGrant>,
    backend: &PluginBackend,
) -> Result<(), String> {
    game.config_targets = ledger.config_targets();
    game.plugin = Some(PluginAssignment {
        plugin_id: revision.plugin_id.clone(),
        revision: PluginRevision {
            version: revision.plugin_version.clone(),
            sha256: revision.package_sha256.clone(),
        },
        update_policy: PluginUpdatePolicy::for_tracking(selection.track_latest),
        allow_prerelease: selection.allow_prerelease,
        copy_zero_matches_to_game_folder: selection.copy_zero_matches_to_game_folder,
        search_directory_destinations: selection.search_directory_destinations,
        assignment_id: assignment_id.clone(),
        resolved_launcher: ledger.resolved_launcher.clone(),
        resolved_watch_target: ledger.resolved_watch_target.clone(),
        grants,
        ledger_reference: backend
            .deployment
            .store()
            .ledger_path(game.id.0, &assignment_id)?,
    });
    Ok(())
}

pub(super) fn rollback_new_assignment_after_config_error(
    backend: &PluginBackend,
    game_id: GameId,
    assignment_id: &str,
    ledger: &crate::plugin::deployment::AssignmentLedger,
    config_error: String,
) -> String {
    let decisions = ledger
        .files
        .iter()
        .map(|file| {
            (
                crate::plugin::deployment::decision_key(&file.destination),
                ModificationDecision::RestoreOriginal,
            )
        })
        .collect();
    match backend.deployment.remove(
        game_id.0,
        assignment_id,
        &decisions,
        || Ok(()),
        crate::plugin::deployment::FailureInjection::default(),
    ) {
        Ok(_) => t!("plugins.error.deploy_commit_restored", error => config_error).to_string(),
        Err(rollback_error) => t!(
            "plugins.error.deploy_commit_unrecovered",
            error => config_error,
            recovery => rollback_error
        )
        .to_string(),
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/app/worker/plugin_actions/execute__tests.rs"]
mod tests;
