use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc;

use eframe::egui;
use rust_i18n::t;

use super::execute::apply_new_assignment;
use crate::gui::main_window::app::worker::messages::WorkerResult;
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::gui::main_window::app::worker::plugin_runtime::{
    run_optional_runtime, runtime_for_ledger, runtime_spec_for_supported_operation,
};
use crate::launcher_gui::config;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::LauncherConfig;
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::deployment::ModificationDecision;
use crate::plugin::runtime::Operation as RuntimeOperation;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_approved_update(
    config_path: &Path,
    mut config: LauncherConfig,
    game_index: usize,
    selection: PluginSelection,
    revision: &crate::plugin::catalog::CatalogRevision,
    manifest: &crate::plugin::manifest::Manifest,
    plan: crate::plugin::paths::DeploymentPlan,
    grants: Vec<crate::plugin::paths::ExternalGrant>,
    decisions: &BTreeMap<String, ModificationDecision>,
    backend: &PluginBackend,
    session_root: &Path,
    game_state: &crate::gui::common::game_state::GameStateHub,
    results: &mpsc::Sender<WorkerResult>,
    ctx: &egui::Context,
    log: &LauncherLog,
) -> Result<LauncherConfig, String> {
    let console = config.settings.console();
    let assignment_id = config
        .games
        .get(game_index)
        .and_then(|game| game.plugin.as_ref())
        .ok_or_else(|| t!("plugins.error.assignment_changed").to_string())?
        .assignment_id
        .clone();
    let mut ledger = backend.deployment.deploy(
        &assignment_id,
        &revision.plugin_version.to_string(),
        &plan,
        decisions,
        crate::plugin::deployment::FailureInjection::default(),
    )?;
    apply_new_assignment(
        &mut config.games[game_index],
        revision,
        &selection,
        assignment_id,
        &ledger,
        grants,
        backend,
    )?;
    config::save_atomic(config_path, &config)
        .map_err(|error| t!("plugins.error.update_commit_failed", error => error).to_string())?;
    if let Err(error) = crate::plugin::lifecycle::install_scripts::run_pending(
        backend.deployment.store(),
        &revision.root,
        manifest,
        &mut ledger,
    ) {
        let _ = results.send(WorkerResult::ConfigCommitted(config.clone()));
        ctx.request_repaint();
        return Err(t!("plugins.error.install_script_failed", error => error).to_string());
    }
    let game = config.games[game_index].clone();
    if let Some(runtime) = runtime_for_ledger(manifest, &ledger)? {
        let spec = runtime_spec_for_supported_operation(
            &game,
            &ledger,
            Some(&runtime),
            RuntimeOperation::Prepare,
            session_root,
            console,
            ctx,
        )?;
        if let Err(error) = run_optional_runtime(game.id, spec, game_state, results, ctx, log) {
            let _ = results.send(WorkerResult::ConfigCommitted(config.clone()));
            ctx.request_repaint();
            return Err(
                t!("plugins.error.prepare_after_deploy_failed", error => error).to_string(),
            );
        }
    }
    Ok(config)
}
