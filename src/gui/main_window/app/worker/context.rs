use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::Arc;

use eframe::egui;

use super::messages::{WorkerCommand, WorkerResult};
use super::pickers::{pick_controller_image, pick_executable};
use super::plugin_actions::PendingPluginAction;
use super::plugin_backend::PluginBackend;
use super::sessions::ActiveWatch;
use crate::gui::common::game_state::GameStateHub;
use crate::launcher_gui::config;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::GameId;
use crate::launcher_gui::process::ProcessManager;
use crate::launcher_gui::runtime::RuntimeService;
use crate::plugin::catalog::Catalog;
use crate::plugin::deployment::DeploymentEngine;
use crate::plugin::lifecycle::LifecycleService;
use crate::plugin::paths::PathPolicy;

pub(super) struct WorkerContext {
    pub(super) launcher_dir: PathBuf,
    pub(super) config_path: PathBuf,
    pub(super) log: LauncherLog,

    pub(super) ctx: egui::Context,
    pub(super) game_state: GameStateHub,
    pub(super) results: Sender<WorkerResult>,

    pub(super) internal_sender: SyncSender<WorkerCommand>,
    pub(super) runtime: RuntimeService,
    pub(super) session_root: PathBuf,
    pub(super) plugin_backend: Result<PluginBackend, String>,
    pub(super) processes: ProcessManager,
    pub(super) active_watches: BTreeMap<GameId, ActiveWatch>,

    pub(super) pending_plugin_action: Option<PendingPluginAction>,
}

impl WorkerContext {
    pub(super) fn new(
        launcher_dir: PathBuf,
        log: LauncherLog,
        ctx: egui::Context,
        game_state: GameStateHub,
        results: Sender<WorkerResult>,
        internal_sender: SyncSender<WorkerCommand>,
    ) -> Self {
        let config_path = launcher_dir.join(config::FILE_NAME);
        let runtime = RuntimeService::new(launcher_dir.clone());
        let catalog_root = launcher_dir.join("plugins");
        let state_root = launcher_dir.join("plugin_state");
        let session_root = state_root.join("sessions");
        let plugin_backend = (|| {
            let catalog = Catalog::open(&catalog_root)?;
            let deployment = DeploymentEngine::new(
                &state_root,
                PathPolicy {
                    application_root: launcher_dir.clone(),
                    catalog_root: catalog_root.clone(),
                    state_root: state_root.clone(),
                },
            )?;
            let lifecycle = LifecycleService::new(catalog.clone(), deployment.clone())?;
            Ok::<_, String>(PluginBackend {
                catalog,
                deployment,
                lifecycle,
            })
        })();
        let process_sender = internal_sender.clone();
        let processes = ProcessManager::with_completion_sink(Arc::new(move |completion| {
            let _ = process_sender.send(WorkerCommand::ProcessCompletion(completion));
        }));
        let active_watches: BTreeMap<GameId, ActiveWatch> = BTreeMap::new();
        let pending_plugin_action: Option<PendingPluginAction> = None;
        Self {
            launcher_dir,
            config_path,
            log,
            ctx,
            game_state,
            results,
            internal_sender,
            runtime,
            session_root,
            plugin_backend,
            processes,
            active_watches,
            pending_plugin_action,
        }
    }

    pub(super) fn run(mut self, commands: Receiver<WorkerCommand>) {
        self.announce_catalog();
        while let Ok(command) = commands.recv() {
            let Some(result) = self.handle(command) else {
                break;
            };
            if !matches!(result, WorkerResult::Internal) {
                if self.results.send(result).is_err() {
                    break;
                }

                self.ctx.request_repaint();
            }

            self.emit_completed_games();
        }
    }

    fn handle(&mut self, command: WorkerCommand) -> Option<WorkerResult> {
        Some(match command {
            WorkerCommand::PickExecutable => pick_executable(),
            WorkerCommand::PickControllerImage => pick_controller_image(),
            WorkerCommand::AddGame {
                config,
                game,
                plugin,
            } => self.add_game(config, game, plugin),
            WorkerCommand::RefreshPlugins => self.refresh_plugins(),
            WorkerCommand::ImportPlugin(path) => self.import_package(path),
            WorkerCommand::ExportPlugin { plugin, output } => self.export_plugin(plugin, output),
            WorkerCommand::EditPlugin { plugin, manifest } => self.edit_plugin(plugin, manifest),
            WorkerCommand::RemoveCatalogRevision { plugin, config } => {
                self.remove_catalog_revision(plugin, config)
            }
            WorkerCommand::ApplyGamePlugin {
                config,
                game_id,
                selection,
            } => self.apply_game_plugin(config, game_id, selection),
            WorkerCommand::RemoveGamePlugin { config, game_id } => {
                self.remove_game_plugin(config, game_id)
            }
            WorkerCommand::RepairGamePlugin { config, game_id } => {
                self.repair_game_plugin(config, game_id)
            }
            WorkerCommand::ReResolveGamePlugin { config, game_id } => {
                self.re_resolve_game_plugin(config, game_id)
            }
            WorkerCommand::InspectGamePlugin { game } => self.inspect_plugin(game),
            WorkerCommand::ResetGamePluginConfig { game } => self.reset_plugin_config(game),
            WorkerCommand::ConfirmPluginAction { token, approval } => {
                self.confirm_plugin_action(token, approval)
            }
            WorkerCommand::CancelPluginAction { token } => self.cancel_plugin_action(token),
            WorkerCommand::SaveConfig { config } => self.save_config(config),
            WorkerCommand::SaveConfigQuiet {
                config,
                retire_backgrounds,
            } => self.save_config_quiet(config, retire_backgrounds),
            WorkerCommand::SavePresentation {
                config,
                game_id,
                label,
                source,
                clear_image,
            } => self.save_presentation(config, game_id, label, source, clear_image),
            WorkerCommand::Launch { game, config } => self.launch(game, config),
            WorkerCommand::CloseGame { game } => self.close_game(game),
            WorkerCommand::CloseActiveGames { games } => self.close_active_games(games),
            WorkerCommand::RemoveGame { config } => self.remove_game(config),
            WorkerCommand::Shutdown => {
                self.shutdown();
                return None;
            }
            WorkerCommand::ProcessCompletion(completion) => self.process_completion(completion),
            WorkerCommand::WatchEvent(event) => self.watch_event(event),
            WorkerCommand::PluginRuntimeEvent {
                game_id,
                operation_id,
                event,
            } => self.plugin_runtime_event(game_id, operation_id, event),
        })
    }
}
