mod change;
mod execute;
mod inspection;
pub(super) mod preview;
mod removal;
mod reset;
mod resolution;
mod summary;
mod update;

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig};
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::paths::{PlanPreview, ResolutionChoices};
use crate::plugin::workflow::PluginActionApproval;
use execute::execute_pending_plugin_action;
use inspection::inspect_game_plugin;
use preview::{preview_apply_game_plugin, preview_remove_game_plugin, preview_repair_game_plugin};
use reset::reset_game_plugin_config;

pub(super) struct PendingPluginAction {
    pub(super) token: String,
    pub(super) revision: crate::plugin::catalog::CatalogRevision,
    pub(super) manifest: crate::plugin::manifest::Manifest,
    pub(super) preview: Option<PlanPreview>,
    pub(super) choices: ResolutionChoices,
    pub(super) existing_ledger: Option<crate::plugin::deployment::AssignmentLedger>,
    pub(super) kind: PendingPluginKind,
}

pub(super) enum PendingPluginKind {
    Add {
        config: LauncherConfig,
        game: GameEntry,
        selection: PluginSelection,
        assignment_id: String,
    },
    Install {
        config: LauncherConfig,
        game_index: usize,
        selection: PluginSelection,
        assignment_id: String,
    },
    Update {
        config: LauncherConfig,
        game_index: usize,
        selection: PluginSelection,
    },
    Change {
        config: LauncherConfig,
        game_index: usize,
        selection: PluginSelection,
        old_plan: crate::plugin::paths::DeploymentPlan,
        old_manifest: crate::plugin::manifest::Manifest,
        new_assignment_id: String,
    },
    Repair {
        config: LauncherConfig,
        game_index: usize,
        selection: PluginSelection,
    },
    Remove {
        config: LauncherConfig,
        game_index: usize,
    },
}

impl WorkerContext {
    pub(super) fn apply_game_plugin(
        &mut self,
        config: LauncherConfig,
        game_id: GameId,
        selection: PluginSelection,
    ) -> WorkerResult {
        match preview_apply_game_plugin(config, game_id, selection, &self.plugin_backend) {
            Ok((pending, preview)) => {
                self.pending_plugin_action = Some(pending);
                WorkerResult::PluginActionPreview(preview)
            }
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.preview_deployment").to_string(),
                message,
            },
        }
    }

    pub(super) fn remove_game_plugin(
        &mut self,
        config: LauncherConfig,
        game_id: GameId,
    ) -> WorkerResult {
        match preview_remove_game_plugin(config, game_id, &self.plugin_backend) {
            Ok((pending, preview)) => {
                self.pending_plugin_action = Some(pending);
                WorkerResult::PluginActionPreview(preview)
            }
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.preview_removal").to_string(),
                message,
            },
        }
    }

    pub(super) fn repair_game_plugin(
        &mut self,
        config: LauncherConfig,
        game_id: GameId,
    ) -> WorkerResult {
        match preview_repair_game_plugin(config, game_id, &self.plugin_backend, false) {
            Ok((pending, preview)) => {
                self.pending_plugin_action = Some(pending);
                WorkerResult::PluginActionPreview(preview)
            }
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.preview_repair").to_string(),
                message,
            },
        }
    }

    pub(super) fn re_resolve_game_plugin(
        &mut self,
        config: LauncherConfig,
        game_id: GameId,
    ) -> WorkerResult {
        match preview_repair_game_plugin(config, game_id, &self.plugin_backend, true) {
            Ok((pending, preview)) => {
                self.pending_plugin_action = Some(pending);
                WorkerResult::PluginActionPreview(preview)
            }
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.preview_reresolve").to_string(),
                message,
            },
        }
    }

    pub(super) fn inspect_plugin(&self, game: GameEntry) -> WorkerResult {
        let game_id = game.id;
        WorkerResult::PluginInspection {
            game_id,
            inspection: inspect_game_plugin(&game, &self.plugin_backend),
        }
    }

    pub(super) fn reset_plugin_config(&self, game: GameEntry) -> WorkerResult {
        let game_id = game.id;
        match reset_game_plugin_config(&game, &self.plugin_backend) {
            Ok(reset) => WorkerResult::PluginConfigReset {
                game_id,
                restored: reset.restored,
                backups: reset.backups,
            },
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.config_reset").to_string(),
                message,
            },
        }
    }

    pub(super) fn confirm_plugin_action(
        &mut self,
        token: String,
        approval: PluginActionApproval,
    ) -> WorkerResult {
        match self.pending_plugin_action.take() {
            Some(pending) if pending.token == token => {
                match execute_pending_plugin_action(
                    &self.config_path,
                    pending,
                    approval,
                    &self.plugin_backend,
                    &self.session_root,
                    &self.game_state,
                    &self.results,
                    &self.ctx,
                    &self.log,
                ) {
                    Ok(config) => WorkerResult::ConfigCommitted(config),
                    Err(message) => WorkerResult::Error {
                        title: t!("plugins.error.operation").to_string(),
                        message,
                    },
                }
            }
            Some(pending) => {
                self.pending_plugin_action = Some(pending);
                WorkerResult::Error {
                    title: t!("plugins.error.preview_expired").to_string(),
                    message: t!("plugins.error.approval_mismatch").to_string(),
                }
            }
            None => WorkerResult::Error {
                title: t!("plugins.error.preview_expired").to_string(),
                message: t!("plugins.error.no_pending_preview").to_string(),
            },
        }
    }

    pub(super) fn cancel_plugin_action(&mut self, token: String) -> WorkerResult {
        if self
            .pending_plugin_action
            .as_ref()
            .is_some_and(|pending| pending.token == token)
        {
            self.pending_plugin_action = None;
        }
        WorkerResult::Internal
    }
}
