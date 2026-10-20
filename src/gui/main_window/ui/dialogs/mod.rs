use std::path::PathBuf;

use eframe::egui;
use rust_i18n::t;

use crate::gui::common::components::ModalDialog;
use crate::launcher_gui::hotkeys::Hotkey;
use crate::launcher_gui::model::{CloseMode, GameEntry, GameId};

mod add;
mod config_notice;
mod drafts;
mod game_settings;
mod hotkeys;
mod lifecycle;
mod manifest_editor;
mod message;
mod plugin_action;
mod plugin_choice;
mod plugin_labels;
mod plugin_library;
mod plugin_options;
mod watch_field;

pub use config_notice::config_notice;
pub use drafts::{
    AddDraft, GameSettingsDraft, HotkeyDraft, HotkeyRow, ManifestEditorDraft, PluginActionDraft,
    PluginInspectionState, PluginLibraryDraft, PresentationDraft, StorePicker,
};
pub use plugin_choice::PluginChoice;

#[derive(Clone)]
pub enum Dialog {
    Add(AddDraft),
    Remove(GameEntry),
    Presentation(PresentationDraft),
    Hotkeys(HotkeyDraft),
    GameSettings(GameSettingsDraft),
    ConfigMenu(GameEntry),
    PluginLibrary(PluginLibraryDraft),
    ManifestEditor(Box<ManifestEditorDraft>),
    PluginAction(PluginActionDraft),
    Error { title: String, message: String },
    Notice { title: String, message: String },
}

#[derive(Clone, Debug)]
pub enum DialogEvent {
    None,
    Cancel,
    AddContinue(AddDraft),
    RemoveGame(GameEntry),
    ChooseControllerImage,
    SavePresentation {
        id: GameId,
        label: String,
        source: Option<PathBuf>,
        clear_image: bool,
    },
    SaveHotkeys {
        launcher: Option<Hotkey>,
        launcher_close: Option<Hotkey>,
        games: Vec<(GameId, Option<Hotkey>, Option<Hotkey>)>,
    },
    SaveGameSettings {
        id: GameId,
        watch_exe: Option<String>,
        close_mode: CloseMode,
        close_script: Option<PathBuf>,
        config_targets: Vec<PathBuf>,
    },
    OpenConfigTarget(PathBuf),
    RequestStoreApps,
    RefreshPlugins,
    ImportPlugin(PathBuf),
    ExportPlugin {
        plugin: crate::plugin::catalog::library::PluginDescriptor,
        output: PathBuf,
    },
    RevealPlugin(PathBuf),
    EditPlugin(crate::plugin::catalog::library::PluginDescriptor),
    SaveEditedPlugin {
        plugin: crate::plugin::catalog::library::PluginDescriptor,
        manifest: String,
    },
    RemoveCatalogRevision(crate::plugin::catalog::library::PluginDescriptor),
    ConfirmPluginAction {
        token: String,
        approval: crate::plugin::workflow::PluginActionApproval,
    },
    CancelPluginAction(String),
    ApplyGamePlugin {
        id: GameId,
        selection: crate::plugin::catalog::library::PluginSelection,
    },
    RemoveGamePlugin(GameId),
    RepairGamePlugin(GameId),
    ReResolveGamePlugin(GameId),

    ResetGamePluginConfig(GameId),
}

impl Dialog {
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn notice(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Notice {
            title: title.into(),
            message: message.into(),
        }
    }

    pub fn show(&mut self, context: &egui::Context) -> DialogEvent {
        let mut event = DialogEvent::None;
        let title: String = match self {
            Self::Add(_) => t!("title.add_game").to_string(),
            Self::Remove(_) => t!("title.remove_game").to_string(),
            Self::Presentation(_) => t!("title.edit_label").to_string(),
            Self::Hotkeys(_) => t!("title.hotkeys").to_string(),
            Self::GameSettings(_) => t!("title.game_settings").to_string(),
            Self::ConfigMenu(_) => t!("title.open_config").to_string(),
            Self::PluginLibrary(_) => t!("title.plugins").to_string(),
            Self::ManifestEditor(_) => t!("title.plugin_manifest_editor").to_string(),
            Self::PluginAction(_) => t!("title.review_plugin_changes").to_string(),
            Self::Error { title, .. } | Self::Notice { title, .. } => title.clone(),
        };
        let modal = ModalDialog::new(title, egui::Id::new("launcher-themed-modal"));
        match self {
            Self::Add(draft) => modal.show(context, &mut add::AddDialog::new(draft, &mut event)),
            Self::Remove(game) => {
                modal.show(context, &mut lifecycle::RemoveDialog::new(game, &mut event))
            }
            Self::Presentation(draft) => modal.show(
                context,
                &mut lifecycle::PresentationDialog::new(draft, &mut event),
            ),
            Self::Hotkeys(draft) => {
                modal.show(context, &mut hotkeys::HotkeysDialog::new(draft, &mut event))
            }
            Self::GameSettings(draft) => modal.show(
                context,
                &mut game_settings::GameSettingsDialog::new(draft, &mut event),
            ),
            Self::ConfigMenu(game) => modal.show(
                context,
                &mut lifecycle::ConfigMenuDialog::new(game, &mut event),
            ),
            Self::PluginLibrary(draft) => modal.show(
                context,
                &mut plugin_library::PluginLibraryDialog::new(draft, &mut event),
            ),
            Self::ManifestEditor(draft) => modal.show(
                context,
                &mut manifest_editor::ManifestEditorDialog::new(draft, &mut event),
            ),
            Self::PluginAction(draft) => modal.show(
                context,
                &mut plugin_action::PluginActionDialog::new(draft, &mut event),
            ),
            Self::Error {
                title,
                message: body,
            } => modal.show(
                context,
                &mut message::MessageDialog::new(title, body, message::Tone::Error, &mut event),
            ),
            Self::Notice {
                title,
                message: body,
            } => modal.show(
                context,
                &mut message::MessageDialog::new(title, body, message::Tone::Notice, &mut event),
            ),
        }
        event
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/mod__harness.rs"]
mod harness;

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/mod__tests.rs"]
mod tests;
