use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::plugin::paths::ExternalGrant;
use crate::support::paths::has_extension;
use crate::support::process::console_policy::ConsoleSettings;
use crate::support::process::image::is_script;

pub fn is_launchable_extension(path: &Path) -> bool {
    has_extension(path, "exe") || is_script(path)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GameId(pub u64);

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CloseMode {
    #[default]
    KillExe,

    Script,
}

impl fmt::Display for CloseMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::KillExe => "kill",
            Self::Script => "script",
        })
    }
}

impl FromStr for CloseMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "kill" | "kill-exe" | "killexe" => Ok(Self::KillExe),
            "script" | "bat" => Ok(Self::Script),
            _ => Err(format!(
                "unknown close mode '{value}' (expected kill or script)"
            )),
        }
    }
}

enum WatchSource<'a> {
    Named(&'a str),

    Plugin(&'a Path),

    Automatic(&'a str),
}

pub fn automatic_watch_name(target: &Path) -> Option<String> {
    let is_executable = target
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    if !is_executable {
        return None;
    }
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameEntry {
    pub id: GameId,
    pub label: String,

    pub controller_image: Option<PathBuf>,
    pub selected_target: PathBuf,

    pub plugin: Option<PluginAssignment>,

    pub hotkey: Option<crate::launcher_gui::hotkeys::Hotkey>,

    pub watch_exe: Option<String>,

    pub close_mode: CloseMode,

    pub close_script: Option<PathBuf>,

    pub close_hotkey: Option<crate::launcher_gui::hotkeys::Hotkey>,

    pub config_targets: Vec<PathBuf>,
}

impl GameEntry {
    pub fn game_dir(&self) -> Result<&std::path::Path, String> {
        self.selected_target.parent().ok_or_else(|| {
            format!(
                "registered executable has no parent directory: {}",
                self.selected_target.display()
            )
        })
    }

    pub fn watch_target(&self) -> Option<String> {
        match self.watch_source()? {
            WatchSource::Named(name) | WatchSource::Automatic(name) => Some(name.to_string()),
            WatchSource::Plugin(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned),
        }
    }

    pub fn watch_target_path(&self) -> Option<PathBuf> {
        match self.watch_source()? {
            WatchSource::Named(name) => {
                let path = std::path::Path::new(name);
                if path.is_absolute() {
                    return Some(path.to_path_buf());
                }
                let has_directory = path
                    .parent()
                    .is_some_and(|parent| !parent.as_os_str().is_empty());
                if has_directory {
                    Some(self.game_dir().ok()?.join(path))
                } else {
                    None
                }
            }
            WatchSource::Plugin(path) => Some(path.to_path_buf()),
            WatchSource::Automatic(_) => None,
        }
    }

    pub fn stored_automatic_watch(&self) -> Option<String> {
        self.stored_watch()
            .filter(|name| self.is_automatic_watch(name))
            .map(ToOwned::to_owned)
    }

    fn stored_watch(&self) -> Option<&str> {
        self.watch_exe
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn is_automatic_watch(&self, name: &str) -> bool {
        automatic_watch_name(&self.selected_target)
            .is_some_and(|automatic| automatic.eq_ignore_ascii_case(name))
    }

    fn watch_source(&self) -> Option<WatchSource<'_>> {
        let stored = self.stored_watch();
        let plugin = self
            .plugin
            .as_ref()
            .and_then(|assignment| assignment.resolved_watch_target.as_deref());
        match (stored, plugin) {
            (Some(name), _) if !self.is_automatic_watch(name) => Some(WatchSource::Named(name)),
            (_, Some(path)) => Some(WatchSource::Plugin(path)),
            (Some(name), None) => Some(WatchSource::Automatic(name)),
            (None, None) => None,
        }
    }

    pub fn launch_target(&self) -> &Path {
        self.plugin
            .as_ref()
            .and_then(|assignment| assignment.resolved_launcher.as_deref())
            .unwrap_or(&self.selected_target)
    }

    pub fn close_plan(&self) -> ClosePlan {
        match self.close_mode {
            CloseMode::Script => match &self.close_script {
                Some(script) => ClosePlan::RunScript(script.clone()),
                None => ClosePlan::Nothing,
            },
            CloseMode::KillExe => {
                if let Some(watch) = self
                    .watch_exe
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    ClosePlan::Kill(watch.to_string())
                } else if !is_script(&self.selected_target) {
                    match self
                        .selected_target
                        .file_name()
                        .and_then(|name| name.to_str())
                    {
                        Some(name) => ClosePlan::Kill(name.to_string()),
                        None => ClosePlan::Nothing,
                    }
                } else {
                    ClosePlan::Nothing
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginUpdatePolicy {
    TrackLatest,
    Pinned,
}

impl PluginUpdatePolicy {
    pub fn for_tracking(track_latest: bool) -> Self {
        if track_latest {
            Self::TrackLatest
        } else {
            Self::Pinned
        }
    }

    pub fn tracks_latest(self) -> bool {
        self == Self::TrackLatest
    }
}

impl fmt::Display for PluginUpdatePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TrackLatest => "track_latest",
            Self::Pinned => "pinned",
        })
    }
}

impl FromStr for PluginUpdatePolicy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "track_latest" => Ok(Self::TrackLatest),
            "pinned" => Ok(Self::Pinned),
            _ => Err(format!(
                "unknown plugin update policy '{value}' (expected track_latest or pinned)"
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginRevision {
    pub version: semver::Version,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginAssignment {
    pub plugin_id: String,
    pub revision: PluginRevision,
    pub update_policy: PluginUpdatePolicy,

    pub allow_prerelease: bool,

    pub copy_zero_matches_to_game_folder: bool,

    pub search_directory_destinations: bool,
    pub assignment_id: String,
    pub resolved_launcher: Option<PathBuf>,
    pub resolved_watch_target: Option<PathBuf>,
    pub grants: Vec<ExternalGrant>,
    pub ledger_reference: PathBuf,
}

impl PluginAssignment {
    pub fn validate(&self) -> Result<(), String> {
        if self.plugin_id.trim().is_empty() {
            return Err("plugin assignment has an empty plugin ID".into());
        }
        if self.revision.sha256.len() != 64
            || !self
                .revision
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("plugin assignment SHA-256 is invalid".into());
        }
        if self.assignment_id.len() != 32
            || !self
                .assignment_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("plugin assignment ID is invalid".into());
        }
        if !self.ledger_reference.is_absolute() {
            return Err("plugin ledger reference must be absolute".into());
        }
        for (field, path) in [
            ("resolved launcher", self.resolved_launcher.as_deref()),
            (
                "resolved watch target",
                self.resolved_watch_target.as_deref(),
            ),
        ] {
            if path.is_some_and(|path| !path.is_absolute()) {
                return Err(format!("plugin {field} must be absolute"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosePlan {
    RunScript(PathBuf),

    Kill(String),

    Nothing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherSettings {
    pub auto_hide_on_launch: bool,
    pub show_command_windows: bool,
    pub launch_default_on_open: bool,

    pub diagnostics_logging: bool,

    pub block_exit_on_credit: bool,

    pub pause_on_failure: bool,

    pub language: String,

    pub adapter: Option<u32>,
    pub launcher_hotkey: Option<crate::launcher_gui::hotkeys::Hotkey>,

    pub close_game_hotkey: Option<crate::launcher_gui::hotkeys::Hotkey>,
    pub controller_screen: crate::game_launcher_screen::ControllerScreenConfig,
}

impl LauncherSettings {
    pub fn console(&self) -> ConsoleSettings {
        ConsoleSettings {
            show_command_windows: self.show_command_windows,
            pause_on_failure: self.pause_on_failure,
        }
    }
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            auto_hide_on_launch: true,
            show_command_windows: false,
            launch_default_on_open: false,
            diagnostics_logging: false,
            block_exit_on_credit: false,
            pause_on_failure: false,
            language: String::new(),
            adapter: None,
            launcher_hotkey: Some(crate::launcher_gui::hotkeys::default_launcher_hotkey()),
            close_game_hotkey: None,
            controller_screen: crate::game_launcher_screen::ControllerScreenConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherConfig {
    pub settings: LauncherSettings,
    pub default_game: Option<GameId>,
    pub next_game_id: u64,
    pub games: Vec<GameEntry>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            settings: LauncherSettings::default(),
            default_game: None,
            next_game_id: 1,
            games: Vec::new(),
        }
    }
}

impl LauncherConfig {
    pub fn allocate_id(&mut self) -> GameId {
        let id = GameId(self.next_game_id.max(1));
        self.next_game_id = id.0.saturating_add(1);
        id
    }

    pub fn game(&self, id: GameId) -> Option<&GameEntry> {
        self.games.iter().find(|game| game.id == id)
    }

    pub fn set_default(&mut self, id: Option<GameId>) -> Result<(), String> {
        if let Some(id) = id {
            if self.game(id).is_none() {
                return Err(format!("cannot select missing game {id} as default"));
            }
        }
        self.default_game = id;
        Ok(())
    }

    pub fn remove_game(&mut self, id: GameId) -> Option<GameEntry> {
        let index = self.games.iter().position(|game| game.id == id)?;
        let removed = self.games.remove(index);
        if self.default_game == Some(id) {
            self.default_game = None;
        }
        Some(removed)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        let mut max_id = 0;
        for game in &self.games {
            if game.id.0 == 0 || !ids.insert(game.id) {
                return Err(format!("duplicate or invalid stable game id {}", game.id));
            }
            if game.label.trim().is_empty() {
                return Err(format!("game {} has an empty label", game.id));
            }
            if let Some(image) = &game.controller_image {
                let portable = image.is_relative()
                    && !image.components().any(|component| {
                        matches!(
                            component,
                            std::path::Component::ParentDir
                                | std::path::Component::RootDir
                                | std::path::Component::Prefix(_)
                        )
                    });
                if !portable {
                    return Err(format!(
                        "game {} controller image must be a portable relative path: {}",
                        game.id,
                        image.display()
                    ));
                }
            }
            if !game.selected_target.is_absolute() {
                return Err(format!(
                    "game {} executable path is not absolute: {}",
                    game.id,
                    game.selected_target.display()
                ));
            }
            if !is_launchable_extension(&game.selected_target) {
                return Err(format!(
                    "game {} target is not an .exe, .bat, or .cmd: {}",
                    game.id,
                    game.selected_target.display()
                ));
            }
            if let Some(plugin) = &game.plugin {
                plugin.validate().map_err(|error| {
                    format!("game {} has an invalid plugin assignment: {error}", game.id)
                })?;
            }
            max_id = max_id.max(game.id.0);
        }
        if self.next_game_id <= max_id {
            return Err(format!(
                "next_game_id {} must be greater than the largest game id {}",
                self.next_game_id, max_id
            ));
        }
        if let Some(default) = self.default_game {
            if !ids.contains(&default) {
                return Err(format!(
                    "default_game {default} does not name a registered game"
                ));
            }
        }
        crate::game_launcher_screen::validate_screen_config(&self.settings.controller_screen)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/domain/model__tests.rs"]
mod tests;
