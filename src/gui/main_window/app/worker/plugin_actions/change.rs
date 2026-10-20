use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc;

use eframe::egui;
use rust_i18n::t;

use super::execute::apply_new_assignment;
use crate::gui::main_window::app::worker::messages::WorkerResult;
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::gui::main_window::app::worker::plugin_runtime::{
    run_optional_runtime, runtime_for_ledger, runtime_for_plan, runtime_spec_for_planned_operation,
    runtime_spec_for_supported_operation,
};
use crate::launcher_gui::config;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::LauncherConfig;
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::deployment::{AssignmentState, ModificationDecision};
use crate::plugin::runtime::Operation as RuntimeOperation;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_approved_change(
    config_path: &Path,
    mut config: LauncherConfig,
    game_index: usize,
    selection: PluginSelection,
    new_revision: &crate::plugin::catalog::CatalogRevision,
    new_manifest: &crate::plugin::manifest::Manifest,
    new_plan: crate::plugin::paths::DeploymentPlan,
    new_grants: Vec<crate::plugin::paths::ExternalGrant>,
    old_plan: crate::plugin::paths::DeploymentPlan,
    old_manifest: crate::plugin::manifest::Manifest,
    new_assignment_id: String,
    old_decisions: &BTreeMap<String, ModificationDecision>,
    backend: &PluginBackend,
    session_root: &Path,
    game_state: &crate::gui::common::game_state::GameStateHub,
    results: &mpsc::Sender<WorkerResult>,
    ctx: &egui::Context,
    log: &LauncherLog,
) -> Result<LauncherConfig, String> {
    let console = config.settings.console();
    let game = config
        .games
        .get(game_index)
        .cloned()
        .ok_or_else(|| t!("plugins.error.game_gone_before_change").to_string())?;
    let existing = game
        .plugin
        .clone()
        .ok_or_else(|| t!("plugins.error.assignment_changed").to_string())?;
    if existing.plugin_id == new_revision.plugin_id {
        return Err(t!("plugins.error.change_preview_mismatch").to_string());
    }
    let old_ledger = backend
        .deployment
        .store()
        .load(game.id.0, &existing.assignment_id)?;
    let old_runtime = runtime_for_ledger(&old_manifest, &old_ledger)?;
    let old_restore = runtime_spec_for_supported_operation(
        &game,
        &old_ledger,
        old_runtime.as_ref(),
        RuntimeOperation::Restore,
        session_root,
        console,
        ctx,
    )?;
    let old_prepare = runtime_spec_for_supported_operation(
        &game,
        &old_ledger,
        old_runtime.as_ref(),
        RuntimeOperation::Prepare,
        session_root,
        console,
        ctx,
    )?;
    let new_runtime = runtime_for_plan(new_manifest, &new_plan)?;
    let new_prepare = runtime_spec_for_planned_operation(
        &game,
        new_revision,
        &new_assignment_id,
        &new_plan,
        new_runtime.as_ref(),
        RuntimeOperation::Prepare,
        session_root,
        console,
        ctx,
    )?;
    let new_restore = runtime_spec_for_planned_operation(
        &game,
        new_revision,
        &new_assignment_id,
        &new_plan,
        new_runtime.as_ref(),
        RuntimeOperation::Restore,
        session_root,
        console,
        ctx,
    )?;
    let change_old_prepare = old_prepare.clone();
    let change_new_restore = new_restore.clone();
    let change = backend
        .deployment
        .change_plugin(
            &existing.assignment_id,
            &existing.revision.version.to_string(),
            &old_plan,
            old_decisions,
            || run_optional_runtime(game.id, old_restore, game_state, results, ctx, log),
            || run_optional_runtime(game.id, change_old_prepare, game_state, results, ctx, log),
            &new_assignment_id,
            &new_revision.plugin_version.to_string(),
            &new_plan,
            &BTreeMap::new(),
            || {
                let mut ledger = backend
                    .deployment
                    .store()
                    .load(game.id.0, &new_assignment_id)?;
                crate::plugin::lifecycle::install_scripts::run_pending(
                    backend.deployment.store(),
                    &new_revision.root,
                    new_manifest,
                    &mut ledger,
                )?;
                run_optional_runtime(game.id, new_prepare, game_state, results, ctx, log)
            },
            || run_optional_runtime(game.id, change_new_restore, game_state, results, ctx, log),
            crate::plugin::deployment::FailureInjection::default(),
        )
        .map_err(|error| error.to_string())?;
    if change.removed_assignment.state != AssignmentState::Removed {
        return Err(t!("plugins.error.not_removed").to_string());
    }
    apply_new_assignment(
        &mut config.games[game_index],
        new_revision,
        &selection,
        new_assignment_id.clone(),
        &change.active_assignment,
        new_grants,
        backend,
    )?;
    if let Err(config_error) = config::save_atomic(config_path, &config) {
        let removal_decisions = change
            .active_assignment
            .files
            .iter()
            .map(|file| {
                (
                    crate::plugin::deployment::decision_key(&file.destination),
                    ModificationDecision::RestoreOriginal,
                )
            })
            .collect();
        let undo_new = backend.deployment.remove(
            game.id.0,
            &new_assignment_id,
            &removal_decisions,
            || run_optional_runtime(game.id, new_restore, game_state, results, ctx, log),
            crate::plugin::deployment::FailureInjection::default(),
        );
        let restore_old = undo_new.and_then(|_| {
            backend.deployment.restore_removed_assignment(
                &existing.assignment_id,
                &existing.revision.version.to_string(),
                &old_plan,
                || run_optional_runtime(game.id, old_prepare, game_state, results, ctx, log),
            )
        });
        return Err(match restore_old {
            Ok(()) => t!("plugins.error.change_commit_restored", error => config_error).to_string(),
            Err(recovery_error) => t!(
                "plugins.error.change_commit_unrecovered",
                error => config_error,
                recovery => recovery_error
            )
            .to_string(),
        });
    }
    Ok(config)
}
