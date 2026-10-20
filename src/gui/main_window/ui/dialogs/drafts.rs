use std::path::{Path, PathBuf};

use rust_i18n::t;

use crate::launcher_gui::hotkeys::Hotkey;
use crate::launcher_gui::model::{automatic_watch_name, CloseMode, GameEntry, GameId};
use crate::plugin::catalog::library::PluginDescriptor;
use crate::support::paths::paths_equal;

use super::plugin_choice::PluginChoice;

#[derive(Clone, Debug)]
pub struct AddDraft {
    pub path: PathBuf,
    pub label: String,
    pub plugin_choice: PluginChoice,

    pub watch_exe: String,

    watch_auto: Option<String>,

    pub label_focused: bool,
}

pub fn watch_to_store(
    typed: &str,
    automatic: Option<&str>,
    resolved: Option<&str>,
    stored_automatic: Option<&str>,
) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return None;
    }
    if resolved.is_some_and(|resolved| resolved.eq_ignore_ascii_case(typed)) {
        return stored_automatic.map(ToOwned::to_owned);
    }
    if automatic.is_some_and(|automatic| automatic.eq_ignore_ascii_case(typed)) {
        return Some(typed.to_string());
    }
    Some(typed.to_string())
}

impl AddDraft {
    pub fn new(path: PathBuf, label: String, plugins: Vec<PluginDescriptor>) -> Self {
        let watch_auto = automatic_watch_name(&path);
        Self {
            watch_exe: watch_auto.clone().unwrap_or_default(),
            watch_auto,
            path,
            label,
            plugin_choice: PluginChoice::new(plugins),
            label_focused: false,
        }
    }

    pub fn watch_default(&self) -> Option<String> {
        self.resolved_watch_name()
            .or_else(|| automatic_watch_name(&self.path))
    }

    pub fn sync_watch_default(&mut self) {
        let default = self.watch_default();
        if default == self.watch_auto {
            return;
        }
        if self.watch_exe.trim() == self.watch_auto.as_deref().unwrap_or("") {
            self.watch_exe = default.clone().unwrap_or_default();
        }
        self.watch_auto = default;
    }

    pub fn resolved_watch_name(&self) -> Option<String> {
        let plugin = self.plugin_choice.selected_plugin()?;
        let requested = plugin
            .manifest
            .executable_watch
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let candidate = match requested {
            Some(value) => Path::new(value).file_name(),
            None => self.path.file_name(),
        };
        candidate.and_then(|name| name.to_str()).map(str::to_owned)
    }

    pub fn watch_override(&self) -> Option<String> {
        watch_to_store(
            &self.watch_exe,
            automatic_watch_name(&self.path).as_deref(),
            self.resolved_watch_name().as_deref(),
            None,
        )
    }

    pub fn game_entry(&self, id: GameId) -> GameEntry {
        GameEntry {
            id,
            label: self.label.trim().to_string(),
            controller_image: None,
            selected_target: self.path.clone(),
            plugin: None,
            hotkey: None,
            watch_exe: self.watch_override(),
            close_mode: CloseMode::KillExe,
            close_script: None,
            close_hotkey: None,
            config_targets: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PluginLibraryDraft {
    pub plugins: Vec<crate::plugin::catalog::library::PluginDescriptor>,
    pub selected: Option<usize>,
    pub notice: Option<String>,
    pub confirm_remove: bool,
    pub assigned_games: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct PluginActionDraft {
    pub preview: crate::plugin::workflow::PluginActionPreview,
    pub approval: crate::plugin::workflow::PluginActionApproval,
}

impl PluginActionDraft {
    pub fn new(preview: crate::plugin::workflow::PluginActionPreview) -> Self {
        let approval = crate::plugin::workflow::PluginActionApproval {
            launcher_destination: preview.resolved_launcher.clone(),
            watch_destination: preview.resolved_watch_target.clone(),
            ..crate::plugin::workflow::PluginActionApproval::default()
        };
        Self { preview, approval }
    }

    pub fn external_granted(&self, root: &std::path::Path) -> bool {
        self.approval
            .granted_external_roots
            .iter()
            .any(|candidate| paths_equal(candidate, root))
    }

    pub fn set_external_granted(&mut self, root: PathBuf, granted: bool) {
        self.approval
            .granted_external_roots
            .retain(|candidate| !paths_equal(candidate, &root));
        if granted {
            self.approval.granted_external_roots.push(root);
        }
    }

    pub fn is_complete(&self) -> bool {
        use crate::plugin::paths::AmbiguityKind;
        let grants_complete = self
            .preview
            .required_external_grants
            .iter()
            .all(|grant| self.external_granted(&grant.destination_root));
        let selections_complete = self.preview.ambiguities.iter().all(|ambiguity| {
            let selected = match ambiguity.kind {
                AmbiguityKind::Launcher => self.approval.launcher_destination.as_ref(),
                AmbiguityKind::WatchTarget => self.approval.watch_destination.as_ref(),
            };
            selected.is_some_and(|selected| {
                ambiguity
                    .candidates
                    .iter()
                    .any(|candidate| paths_equal(candidate, selected))
            })
        });
        let modifications_complete = self.preview.modifications.iter().all(|modification| {
            let key = crate::plugin::deployment::decision_key(&modification.destination);
            self.approval.modification_decisions.contains_key(&key)
        });
        grants_complete && selections_complete && modifications_complete
    }

    pub fn cancellation_event(&self) -> super::DialogEvent {
        super::DialogEvent::CancelPluginAction(self.preview.token.clone())
    }

    pub fn confirmation_event(&self) -> Option<super::DialogEvent> {
        self.is_complete()
            .then(|| super::DialogEvent::ConfirmPluginAction {
                token: self.preview.token.clone(),
                approval: self.approval.clone(),
            })
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/drafts__plugin_action_tests.rs"]
mod plugin_action_tests;

impl PluginLibraryDraft {
    pub fn new(
        plugins: Vec<crate::plugin::catalog::library::PluginDescriptor>,
        notice: Option<String>,
        games: &[GameEntry],
    ) -> Self {
        let assigned_games = assignment_map(&plugins, games);
        Self {
            selected: (!plugins.is_empty()).then_some(0),
            plugins,
            notice,
            confirm_remove: false,
            assigned_games,
        }
    }

    pub fn replace_plugins(
        &mut self,
        plugins: Vec<crate::plugin::catalog::library::PluginDescriptor>,
    ) {
        let prior = self
            .selected
            .and_then(|index| self.plugins.get(index))
            .map(|plugin| {
                (
                    plugin.revision.plugin_id.clone(),
                    plugin.revision.plugin_version.clone(),
                    plugin.revision.package_sha256.clone(),
                )
            });
        self.plugins = plugins;
        self.selected = prior
            .and_then(|(id, version, sha)| {
                self.plugins.iter().position(|plugin| {
                    plugin.revision.plugin_id == id
                        && plugin.revision.plugin_version == version
                        && plugin.revision.package_sha256 == sha
                })
            })
            .or_else(|| (!self.plugins.is_empty()).then_some(0));
        self.confirm_remove = false;
    }

    pub fn refresh_assignments(&mut self, games: &[GameEntry]) {
        self.assigned_games = assignment_map(&self.plugins, games);
    }
}

fn assignment_map(
    plugins: &[crate::plugin::catalog::library::PluginDescriptor],
    games: &[GameEntry],
) -> std::collections::BTreeMap<String, Vec<String>> {
    plugins
        .iter()
        .map(|plugin| {
            let key = plugin.revision.package_sha256.clone();
            let labels = games
                .iter()
                .filter(|game| {
                    game.plugin.as_ref().is_some_and(|assignment| {
                        assignment.plugin_id == plugin.revision.plugin_id
                            && assignment.revision.version == plugin.revision.plugin_version
                            && assignment
                                .revision
                                .sha256
                                .eq_ignore_ascii_case(&plugin.revision.package_sha256)
                    })
                })
                .map(|game| game.label.clone())
                .collect();
            (key, labels)
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct ManifestEditorDraft {
    pub source: crate::plugin::catalog::library::PluginDescriptor,
    pub manifest: crate::plugin::manifest::Manifest,
    pub raw: String,
    pub version_text: String,
    pub advanced: bool,
    pub error: Option<String>,
}

impl ManifestEditorDraft {
    pub fn new(source: crate::plugin::catalog::library::PluginDescriptor) -> Result<Self, String> {
        let manifest = source.manifest.clone();
        let raw = manifest.canonical_toml()?;
        Ok(Self {
            version_text: manifest.plugin_version.to_string(),
            source,
            manifest,
            raw,
            advanced: false,
            error: None,
        })
    }

    pub fn refresh_raw(&mut self) {
        match self.manifest.canonical_toml() {
            Ok(raw) => {
                self.raw = raw;
                self.error = None;
            }
            Err(error) => {
                self.error = Some(t!("plugins.editor.invalid", detail => error).to_string())
            }
        }
    }

    pub fn validated_raw(&self) -> Result<String, String> {
        let mut manifest = if self.advanced {
            crate::plugin::manifest::Manifest::parse(&self.raw)?
        } else {
            self.manifest.clone()
        };
        if !self.advanced {
            manifest.plugin_version =
                semver::Version::parse(self.version_text.trim()).map_err(|error| {
                    t!("plugins.editor.version_invalid", error => error).to_string()
                })?;
        }
        manifest.canonical_toml()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HotkeyTarget {
    Launcher,

    CloseLauncher,
    GameLaunch(GameId),
    GameClose(GameId),
}

#[derive(Clone, Debug)]
pub struct HotkeyRow {
    pub id: GameId,
    pub label: String,
    pub launch: Option<Hotkey>,
    pub close: Option<Hotkey>,
}

#[derive(Clone, Debug, Default)]
pub struct HotkeyDraft {
    pub launcher: Option<Hotkey>,
    pub launcher_close: Option<Hotkey>,
    pub games: Vec<HotkeyRow>,
    pub capturing: Option<HotkeyTarget>,
    pub error: Option<String>,
}

impl HotkeyDraft {
    pub(super) fn binding(&self, target: HotkeyTarget) -> Option<Hotkey> {
        match target {
            HotkeyTarget::Launcher => self.launcher,
            HotkeyTarget::CloseLauncher => self.launcher_close,
            HotkeyTarget::GameLaunch(id) => self
                .games
                .iter()
                .find(|row| row.id == id)
                .and_then(|row| row.launch),
            HotkeyTarget::GameClose(id) => self
                .games
                .iter()
                .find(|row| row.id == id)
                .and_then(|row| row.close),
        }
    }

    pub(super) fn set(&mut self, target: HotkeyTarget, value: Option<Hotkey>) {
        match target {
            HotkeyTarget::Launcher => self.launcher = value,
            HotkeyTarget::CloseLauncher => self.launcher_close = value,
            HotkeyTarget::GameLaunch(id) => {
                if let Some(row) = self.games.iter_mut().find(|row| row.id == id) {
                    row.launch = value;
                }
            }
            HotkeyTarget::GameClose(id) => {
                if let Some(row) = self.games.iter_mut().find(|row| row.id == id) {
                    row.close = value;
                }
            }
        }
    }

    pub(super) fn conflict(&self, hotkey: Hotkey, exclude: HotkeyTarget) -> Option<String> {
        if exclude != HotkeyTarget::Launcher && self.launcher == Some(hotkey) {
            return Some(t!("hotkeys.open_launcher").to_string());
        }
        if exclude != HotkeyTarget::CloseLauncher && self.launcher_close == Some(hotkey) {
            return Some(t!("hotkeys.close_game").to_string());
        }
        for row in &self.games {
            if HotkeyTarget::GameLaunch(row.id) != exclude && row.launch == Some(hotkey) {
                return Some(t!("hotkeys.launch_named", game => row.label).to_string());
            }
            if HotkeyTarget::GameClose(row.id) != exclude && row.close == Some(hotkey) {
                return Some(t!("hotkeys.close_named", game => row.label).to_string());
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct GameSettingsDraft {
    pub id: GameId,
    pub label: String,
    pub watch_exe: String,
    pub close_mode: CloseMode,
    pub close_script: Option<PathBuf>,
    pub config_targets: Vec<PathBuf>,
    pub store_picker: Option<StorePicker>,
    pub store_filter: String,
    pub plugin: Option<crate::launcher_gui::model::PluginAssignment>,
    pub plugin_choice: PluginChoice,
    pub plugin_inspection: Option<PluginInspectionState>,

    pub confirm_config_reset: bool,

    pub resolved_watch_name: Option<String>,

    pub automatic_watch_name: Option<String>,

    pub stored_automatic_watch: Option<String>,
}

#[derive(Clone, Debug)]
pub enum PluginInspectionState {
    Loading,
    Loaded(crate::plugin::workflow::AssignmentInspection),
    Failed(String),
}

impl GameSettingsDraft {
    pub fn watch_override(&self) -> Option<String> {
        watch_to_store(
            &self.watch_exe,
            self.automatic_watch_name.as_deref(),
            self.resolved_watch_name.as_deref(),
            self.stored_automatic_watch.as_deref(),
        )
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/drafts__plugin_selection_tests.rs"]
mod plugin_selection_tests;

#[derive(Clone)]
pub struct PresentationDraft {
    pub game: GameEntry,
    pub label: String,
    pub selected_source: Option<PathBuf>,
    pub clear_image: bool,
    pub preview: Option<eframe::egui::TextureHandle>,
    pub error: Option<String>,

    pub label_focused: bool,
}

impl PresentationDraft {
    pub fn new(
        game: GameEntry,
        preview: Option<eframe::egui::TextureHandle>,
        error: Option<String>,
    ) -> Self {
        Self {
            label: game.label.clone(),
            game,
            selected_source: None,
            clear_image: false,
            preview,
            error,
            label_focused: false,
        }
    }

    pub fn select_image(&mut self, source: PathBuf, preview: eframe::egui::TextureHandle) {
        self.selected_source = Some(source);
        self.clear_image = false;
        self.preview = Some(preview);
        self.error = None;
    }

    pub fn clear_image(&mut self) {
        self.selected_source = None;
        self.clear_image = true;
        self.preview = None;
        self.error = None;
    }

    pub fn submission(&self) -> Result<(String, Option<PathBuf>, bool), String> {
        let label = self.label.trim().to_string();
        if label.is_empty() {
            return Err(t!("errors.label_empty").to_string());
        }
        Ok((label, self.selected_source.clone(), self.clear_image))
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/drafts__presentation_tests.rs"]
mod presentation_tests;

#[derive(Clone, Debug)]
pub enum StorePicker {
    Loading,
    Loaded(Vec<crate::launcher_gui::store_apps::StoreApp>),
    Failed(String),
}
