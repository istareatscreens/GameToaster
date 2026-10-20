use std::path::PathBuf;

use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig};
use crate::launcher_gui::process::ProcessCompletion;
use crate::plugin::catalog::library::{PluginDescriptor, PluginSelection};
use crate::plugin::runtime::{Message, RuntimeEvent};
use crate::plugin::workflow::{PluginActionApproval, PluginActionPreview};
use crate::support::process::handle::CreationTime;

#[derive(Debug)]
pub(in crate::gui::main_window::app) enum WorkerCommand {
    PickExecutable,
    PickControllerImage,
    AddGame {
        config: LauncherConfig,
        game: GameEntry,
        plugin: Option<PluginSelection>,
    },
    RefreshPlugins,
    ImportPlugin(PathBuf),
    ExportPlugin {
        plugin: PluginDescriptor,
        output: PathBuf,
    },
    EditPlugin {
        plugin: PluginDescriptor,
        manifest: String,
    },
    RemoveCatalogRevision {
        plugin: PluginDescriptor,
        config: LauncherConfig,
    },
    ApplyGamePlugin {
        config: LauncherConfig,
        game_id: GameId,
        selection: PluginSelection,
    },
    RemoveGamePlugin {
        config: LauncherConfig,
        game_id: GameId,
    },
    RepairGamePlugin {
        config: LauncherConfig,
        game_id: GameId,
    },
    ReResolveGamePlugin {
        config: LauncherConfig,
        game_id: GameId,
    },
    InspectGamePlugin {
        game: GameEntry,
    },

    ResetGamePluginConfig {
        game: GameEntry,
    },
    ConfirmPluginAction {
        token: String,
        approval: PluginActionApproval,
    },
    CancelPluginAction {
        token: String,
    },
    SaveConfig {
        config: LauncherConfig,
    },
    SaveConfigQuiet {
        config: LauncherConfig,

        retire_backgrounds: Vec<PathBuf>,
    },
    SavePresentation {
        config: LauncherConfig,
        game_id: GameId,
        label: String,
        source: Option<PathBuf>,
        clear_image: bool,
    },
    Launch {
        game: GameEntry,
        config: LauncherConfig,
    },
    CloseGame {
        game: GameEntry,
    },

    CloseActiveGames {
        games: Vec<GameEntry>,
    },
    RemoveGame {
        config: LauncherConfig,
    },
    Shutdown,
    ProcessCompletion(ProcessCompletion),
    WatchEvent(crate::launcher_gui::process::WatchEvent),
    PluginRuntimeEvent {
        game_id: GameId,
        operation_id: String,
        event: RuntimeEvent,
    },
}

#[derive(Debug)]
pub(in crate::gui::main_window::app) enum WorkerResult {
    Internal,
    FilePicked(Option<PathBuf>),
    ControllerImagePicked(Option<PathBuf>),
    ConfigCommitted(LauncherConfig),
    ConfigSavedQuiet,
    Launched {
        game_id: GameId,
        root_pid: u32,

        root_created: CreationTime,
        updated_config: Option<LauncherConfig>,
    },

    ProgramStarted {
        process_id: u32,
        created: CreationTime,
    },

    PluginMessage {
        game_id: GameId,
        message: Message,
    },

    PluginOperationEnded {
        game_id: GameId,
    },

    PluginRuntimeEnded {
        game_id: GameId,
        end: super::super::runtime_end::RuntimeEnd,
    },
    PluginCatalog {
        plugins: Vec<PluginDescriptor>,
        notice: Option<String>,
    },
    PluginActionPreview(PluginActionPreview),
    PluginInspection {
        game_id: GameId,
        inspection: Result<crate::plugin::workflow::AssignmentInspection, String>,
    },
    PluginConfigReset {
        game_id: GameId,
        restored: Vec<PathBuf>,
        backups: Vec<PathBuf>,
    },
    GameExited(GameId),

    Closed {
        game_id: GameId,
        exit_observed: bool,
    },

    ActiveGamesClosed,
    ShutdownComplete(Result<(), String>),
    Error {
        title: String,
        message: String,
    },
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/messages__tests.rs"]
mod tests;
