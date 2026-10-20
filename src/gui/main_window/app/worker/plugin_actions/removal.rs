use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc;

use eframe::egui;
use rust_i18n::t;

use crate::gui::main_window::app::worker::messages::WorkerResult;
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::gui::main_window::app::worker::plugin_runtime::{
    run_optional_runtime, runtime_for_ledger, runtime_spec_for_supported_operation,
};
use crate::launcher_gui::config;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::LauncherConfig;
use crate::plugin::deployment::ModificationDecision;
use crate::plugin::runtime::Operation as RuntimeOperation;
use crate::support::paths::paths_equal;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_approved_removal(
    config_path: &Path,
    mut config: LauncherConfig,
    game_index: usize,
    revision: &crate::plugin::catalog::CatalogRevision,
    manifest: &crate::plugin::manifest::Manifest,
    decisions: &BTreeMap<String, ModificationDecision>,
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
        .ok_or_else(|| t!("plugins.error.game_gone_before_removal").to_string())?;
    let assignment = game
        .plugin
        .clone()
        .ok_or_else(|| t!("plugins.error.assignment_changed").to_string())?;
    if assignment.plugin_id != revision.plugin_id
        || assignment.revision.version != revision.plugin_version
        || !assignment
            .revision
            .sha256
            .eq_ignore_ascii_case(&revision.package_sha256)
    {
        return Err(t!("plugins.error.removal_preview_changed").to_string());
    }
    let ledger = backend
        .deployment
        .store()
        .load(game.id.0, &assignment.assignment_id)?;
    let restore = runtime_for_ledger(manifest, &ledger)?
        .map(|runtime| {
            runtime_spec_for_supported_operation(
                &game,
                &ledger,
                Some(&runtime),
                RuntimeOperation::Restore,
                session_root,
                console,
                ctx,
            )
        })
        .transpose()?
        .flatten();
    let managed_config_targets = ledger.config_targets();
    backend.deployment.remove(
        game.id.0,
        &assignment.assignment_id,
        decisions,
        || run_optional_runtime(game.id, restore, game_state, results, ctx, log),
        crate::plugin::deployment::FailureInjection::default(),
    )?;
    let stored = config
        .games
        .get_mut(game_index)
        .ok_or_else(|| t!("plugins.error.game_gone_during_removal").to_string())?;
    stored.plugin = None;
    stored.config_targets.retain(|path| {
        !managed_config_targets
            .iter()
            .any(|managed| paths_equal(path, managed))
    });
    config::save_atomic(config_path, &config)
        .map_err(|error| t!("plugins.error.removal_commit_failed", error => error).to_string())?;
    Ok(config)
}
