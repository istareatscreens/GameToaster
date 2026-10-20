use std::path::{Path, PathBuf};

use configparser::ini::{Ini, IniDefault};

use crate::game_launcher_screen::{
    ControllerScreenConfig, LauncherXAction, MonitorCalibration, OrientationProfile,
};
use crate::launcher_gui::hotkeys::{default_launcher_hotkey, Hotkey};
use crate::launcher_gui::model::{
    CloseMode, GameEntry, GameId, LauncherConfig, LauncherSettings, PluginAssignment,
    PluginRevision, PluginUpdatePolicy,
};
use crate::plugin::paths::ExternalGrant;

pub const FILE_NAME: &str = "GameToaster.ini";
pub const SCHEMA_VERSION: u32 = 3;
const LAUNCHER_SECTION: &str = "launcher";
const CONTROLLER_SECTION: &str = "game_launcher_screen";

mod migration;

pub use migration::{open, Notice, ResetReason};

fn read_document(path: &Path) -> Result<Ini, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse_document(&text).map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn parse_document(text: &str) -> Result<Ini, String> {
    let mut ini = ini_parser();
    ini.read(text.strip_prefix('\u{feff}').unwrap_or(text).to_string())?;
    Ok(ini)
}

fn schema_version(ini: &Ini) -> Option<u32> {
    ini.get(LAUNCHER_SECTION, "schema_version")?
        .trim()
        .parse()
        .ok()
}

fn parse_current(ini: &Ini) -> Result<LauncherConfig, String> {
    let settings = LauncherSettings {
        auto_hide_on_launch: get_bool(ini, "auto_hide_on_launch", true)?,
        show_command_windows: get_bool(ini, "show_command_windows", false)?,
        launch_default_on_open: get_bool(ini, "launch_default_on_open", false)?,
        diagnostics_logging: get_bool(ini, "diagnostics_logging", false)?,
        block_exit_on_credit: get_bool(ini, "block_exit_on_credit", false)?,
        pause_on_failure: get_bool(ini, "pause_on_failure", false)?,
        language: ini
            .get(LAUNCHER_SECTION, "language")
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        adapter: optional_u64(ini, "adapter")?.map(|value| value as u32),
        launcher_hotkey: parse_launcher_hotkey(ini)?,
        close_game_hotkey: parse_close_game_hotkey(ini)?,
        controller_screen: parse_controller_screen(ini)?,
    };
    let default_game = optional_u64(ini, "default_game")?.map(GameId);
    let next_game_id = game_id_in_range(
        optional_u64(ini, "next_game_id")?.unwrap_or(1).max(1),
        "[launcher] next_game_id",
    )?;

    let mut games = Vec::new();
    for (section, values) in ini.get_map_ref() {
        let Some(id_text) = section.strip_prefix("game.") else {
            continue;
        };
        let id = id_text
            .parse::<u64>()
            .map_err(|_| format!("invalid game section [{section}]: stable id must be a u64"))
            .and_then(|id| game_id_in_range(id, &format!("game section [{section}] id")))
            .map(GameId)?;
        let required = |key: &str| {
            values
                .get(key)
                .and_then(Clone::clone)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| format!("missing or empty {key} in [{section}]"))
        };
        let optional = |key: &str| {
            values
                .get(key)
                .and_then(Clone::clone)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let parse_hotkey = |key: &str| -> Result<Option<Hotkey>, String> {
            match optional(key) {
                Some(value) => value
                    .parse::<Hotkey>()
                    .map(Some)
                    .map_err(|error| format!("invalid {key} in [{section}]: {error}")),
                None => Ok(None),
            }
        };
        let hotkey = parse_hotkey("hotkey")?;
        let close_hotkey = parse_hotkey("close_hotkey")?;
        let close_mode = match optional("close_mode") {
            Some(value) => value
                .parse::<CloseMode>()
                .map_err(|error| format!("invalid close_mode in [{section}]: {error}"))?,
            None => CloseMode::default(),
        };

        let mut numbered: Vec<(u32, PathBuf)> = values
            .iter()
            .filter_map(|(key, value)| {
                let index = key.strip_prefix("config_target")?.parse::<u32>().ok()?;
                let path = value.as_ref()?.trim();
                (!path.is_empty()).then(|| (index, PathBuf::from(path)))
            })
            .collect();
        numbered.sort_by_key(|(index, _)| *index);
        let config_targets = numbered.into_iter().map(|(_, path)| path).collect();
        let plugin = parse_plugin_assignment(values, section, id)?;
        games.push(GameEntry {
            id,
            label: required("label")?,
            controller_image: optional("controller_image").map(PathBuf::from),
            selected_target: PathBuf::from(required("selected_target")?),
            plugin,
            hotkey,
            watch_exe: optional("watch_exe"),
            close_mode,
            close_script: optional("close_script").map(PathBuf::from),
            close_hotkey,
            config_targets,
        });
    }
    games.sort_by_key(|game| game.id);

    let config = LauncherConfig {
        settings,
        default_game,
        next_game_id,
        games,
    };
    config.validate()?;
    Ok(config)
}

fn parse_plugin_assignment(
    values: &std::collections::HashMap<String, Option<String>>,
    section: &str,
    game_id: GameId,
) -> Result<Option<PluginAssignment>, String> {
    let optional = |key: &str| {
        values
            .get(key)
            .and_then(Clone::clone)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let Some(plugin_id) = optional("plugin_id") else {
        if values.keys().any(|key| key.starts_with("plugin_")) {
            return Err(format!(
                "incomplete plugin assignment in [{section}]: missing plugin_id"
            ));
        }
        return Ok(None);
    };
    let required = |key: &str| {
        optional(key)
            .ok_or_else(|| format!("incomplete plugin assignment in [{section}]: missing {key}"))
    };
    let version_text = required("plugin_version")?;
    let version = version_text.parse::<semver::Version>().map_err(|error| {
        format!("invalid plugin_version in [{section}] ('{version_text}'): {error}")
    })?;
    let policy_text = required("plugin_update_policy")?;
    let update_policy = policy_text
        .parse::<PluginUpdatePolicy>()
        .map_err(|error| format!("invalid plugin_update_policy in [{section}]: {error}"))?;
    let allow_prerelease = match optional("plugin_allow_prerelease").as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(value) => {
            return Err(format!(
                "invalid plugin_allow_prerelease='{value}' in [{section}] (expected true or false)"
            ))
        }
    };
    let parse_assignment_bool = |key: &str| -> Result<bool, String> {
        match optional(key).as_deref() {
            None | Some("false") => Ok(false),
            Some("true") => Ok(true),
            Some(value) => Err(format!(
                "invalid {key}='{value}' in [{section}] (expected true or false)"
            )),
        }
    };
    let mut numbered = Vec::new();
    for (key, value) in values {
        let Some(index_text) = key.strip_prefix("plugin_grant") else {
            continue;
        };
        let index = index_text
            .parse::<u32>()
            .map_err(|_| format!("invalid plugin grant key '{key}' in [{section}]"))?;
        let raw = value
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("empty {key} in [{section}]"))?;
        let grant = serde_json::from_str::<ExternalGrant>(raw)
            .map_err(|error| format!("invalid {key} in [{section}]: {error}"))?;
        numbered.push((index, grant));
    }
    numbered.sort_by_key(|(index, _)| *index);
    if numbered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(format!("duplicate plugin grant index in [{section}]"));
    }
    let assignment = PluginAssignment {
        plugin_id,
        revision: PluginRevision {
            version,
            sha256: required("plugin_sha256")?,
        },
        update_policy,
        allow_prerelease,
        copy_zero_matches_to_game_folder: parse_assignment_bool(
            "plugin_copy_zero_matches_to_game_folder",
        )?,
        search_directory_destinations: parse_assignment_bool(
            "plugin_search_directory_destinations",
        )?,
        assignment_id: required("plugin_assignment_id")?,
        resolved_launcher: optional("plugin_resolved_launcher").map(PathBuf::from),
        resolved_watch_target: optional("plugin_resolved_watch_target").map(PathBuf::from),
        grants: numbered.into_iter().map(|(_, grant)| grant).collect(),
        ledger_reference: PathBuf::from(required("plugin_ledger_reference")?),
    };
    assignment
        .validate()
        .map_err(|error| format!("invalid plugin assignment in [{section}]: {error}"))?;
    for grant in &assignment.grants {
        if grant.plugin_id != assignment.plugin_id
            || grant.game_id != game_id.0
            || grant.revision_sha256 != assignment.revision.sha256
        {
            return Err(format!(
                "plugin grant binding does not match assignment in [{section}]"
            ));
        }
    }
    Ok(Some(assignment))
}

pub fn save_atomic(path: &Path, config: &LauncherConfig) -> Result<(), String> {
    let mut ini = if path.is_file() {
        read_document(path).map_err(|error| {
            format!(
                "failed to preserve {} while saving: {error}",
                path.display()
            )
        })?
    } else {
        ini_parser()
    };
    let stale_games: Vec<String> = ini
        .get_map_ref()
        .keys()
        .filter(|section| section.starts_with("game."))
        .cloned()
        .collect();
    for section in stale_games {
        ini.remove_section(&section);
    }
    write(&mut ini, path, config)
}

fn save_new(path: &Path, config: &LauncherConfig) -> Result<(), String> {
    write(&mut ini_parser(), path, config)
}

fn write(ini: &mut Ini, path: &Path, config: &LauncherConfig) -> Result<(), String> {
    config.validate()?;
    ini.setstr(
        LAUNCHER_SECTION,
        "schema_version",
        Some(&SCHEMA_VERSION.to_string()),
    );
    set_bool(
        ini,
        "auto_hide_on_launch",
        config.settings.auto_hide_on_launch,
    );
    set_bool(
        ini,
        "show_command_windows",
        config.settings.show_command_windows,
    );
    set_bool(
        ini,
        "diagnostics_logging",
        config.settings.diagnostics_logging,
    );
    set_bool(
        ini,
        "block_exit_on_credit",
        config.settings.block_exit_on_credit,
    );
    set_bool(
        ini,
        "launch_default_on_open",
        config.settings.launch_default_on_open,
    );
    set_bool(ini, "pause_on_failure", config.settings.pause_on_failure);
    ini.setstr(
        LAUNCHER_SECTION,
        "language",
        Some(&config.settings.language),
    );
    ini.setstr(
        LAUNCHER_SECTION,
        "adapter",
        config.settings.adapter.map(|n| n.to_string()).as_deref(),
    );
    ini.setstr(
        LAUNCHER_SECTION,
        "default_game",
        config.default_game.map(|id| id.0.to_string()).as_deref(),
    );
    ini.setstr(
        LAUNCHER_SECTION,
        "next_game_id",
        Some(&config.next_game_id.to_string()),
    );

    ini.setstr(
        LAUNCHER_SECTION,
        "launcher_hotkey",
        Some(
            &config
                .settings
                .launcher_hotkey
                .map(|hotkey| hotkey.to_string())
                .unwrap_or_default(),
        ),
    );
    ini.setstr(
        LAUNCHER_SECTION,
        "close_game_hotkey",
        Some(
            &config
                .settings
                .close_game_hotkey
                .map(|hotkey| hotkey.to_string())
                .unwrap_or_default(),
        ),
    );
    write_controller_screen(ini, &config.settings.controller_screen);
    for game in &config.games {
        let section = format!("game.{}", game.id.0);
        ini.setstr(&section, "label", Some(&game.label));
        if let Some(image) = &game.controller_image {
            ini.setstr(&section, "controller_image", Some(&image.to_string_lossy()));
        }
        ini.setstr(
            &section,
            "selected_target",
            Some(&game.selected_target.to_string_lossy()),
        );
        if let Some(plugin) = &game.plugin {
            ini.setstr(&section, "plugin_id", Some(&plugin.plugin_id));
            ini.setstr(
                &section,
                "plugin_version",
                Some(&plugin.revision.version.to_string()),
            );
            ini.setstr(&section, "plugin_sha256", Some(&plugin.revision.sha256));
            ini.setstr(
                &section,
                "plugin_update_policy",
                Some(&plugin.update_policy.to_string()),
            );
            ini.setstr(
                &section,
                "plugin_allow_prerelease",
                Some(if plugin.allow_prerelease {
                    "true"
                } else {
                    "false"
                }),
            );
            ini.setstr(
                &section,
                "plugin_copy_zero_matches_to_game_folder",
                Some(if plugin.copy_zero_matches_to_game_folder {
                    "true"
                } else {
                    "false"
                }),
            );
            ini.setstr(
                &section,
                "plugin_search_directory_destinations",
                Some(if plugin.search_directory_destinations {
                    "true"
                } else {
                    "false"
                }),
            );
            ini.setstr(
                &section,
                "plugin_assignment_id",
                Some(&plugin.assignment_id),
            );
            ini.setstr(
                &section,
                "plugin_resolved_launcher",
                plugin
                    .resolved_launcher
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .as_deref(),
            );
            ini.setstr(
                &section,
                "plugin_resolved_watch_target",
                plugin
                    .resolved_watch_target
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned())
                    .as_deref(),
            );
            ini.setstr(
                &section,
                "plugin_ledger_reference",
                Some(&plugin.ledger_reference.to_string_lossy()),
            );
            for (index, grant) in plugin.grants.iter().enumerate() {
                let value = serde_json::to_string(grant).map_err(|error| {
                    format!("failed to serialize plugin grant in [{section}]: {error}")
                })?;
                ini.setstr(
                    &section,
                    &format!("plugin_grant{}", index + 1),
                    Some(&value),
                );
            }
        }
        if let Some(hotkey) = game.hotkey {
            ini.setstr(&section, "hotkey", Some(&hotkey.to_string()));
        }
        if let Some(watch_exe) = &game.watch_exe {
            ini.setstr(&section, "watch_exe", Some(watch_exe));
        }
        ini.setstr(&section, "close_mode", Some(&game.close_mode.to_string()));
        if let Some(close_script) = &game.close_script {
            ini.setstr(
                &section,
                "close_script",
                Some(&close_script.to_string_lossy()),
            );
        }
        if let Some(close_hotkey) = game.close_hotkey {
            ini.setstr(&section, "close_hotkey", Some(&close_hotkey.to_string()));
        }
        for (index, target) in game.config_targets.iter().enumerate() {
            ini.setstr(
                &section,
                &format!("config_target{}", index + 1),
                Some(&target.to_string_lossy()),
            );
        }
    }
    crate::support::fs::atomic_write(path, ini.writes().as_bytes())
        .map_err(|error| format!("failed to save configuration {}: {error}", path.display()))
}

fn get_bool(ini: &Ini, key: &str, default: bool) -> Result<bool, String> {
    get_section_bool(ini, LAUNCHER_SECTION, key, default)
}

fn get_section_bool(ini: &Ini, section: &str, key: &str, default: bool) -> Result<bool, String> {
    ini.getboolcoerce(section, key)
        .map_err(|error| format!("invalid [{section}] {key}: {error}"))
        .map(|value| value.unwrap_or(default))
}

fn parse_controller_screen(ini: &Ini) -> Result<ControllerScreenConfig, String> {
    let defaults = ControllerScreenConfig::default();
    let enabled = get_section_bool(ini, CONTROLLER_SECTION, "enabled", defaults.enabled)?;
    let launch_on_start = get_section_bool(
        ini,
        CONTROLLER_SECTION,
        "launch_on_start",
        defaults.launch_on_start,
    )?;
    let driver_id = optional_section(ini, CONTROLLER_SECTION, "controller_driver")
        .unwrap_or_else(|| defaults.controller.driver_id.clone());
    let mut controller = crate::game_launcher_screen::default_controller_config(&driver_id)?;
    if let Some(values) = ini.get_map_ref().get(CONTROLLER_SECTION) {
        for (key, value) in values {
            if let Some(option) = key.strip_prefix("controller_option.") {
                controller
                    .options
                    .insert(option.to_string(), value.clone().unwrap_or_default());
            }
        }
    }
    let orientation = optional_section(ini, CONTROLLER_SECTION, "orientation")
        .map(|value| value.parse::<OrientationProfile>())
        .transpose()?
        .unwrap_or(defaults.orientation);
    let calibration = parse_calibration(ini, orientation)?;
    let value = ControllerScreenConfig {
        enabled,
        launch_on_start,
        reopen_after_game_close: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "reopen_after_game_close",
            defaults.reopen_after_game_close,
        )?,
        reopen_on_monitor_return: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "reopen_on_monitor_return",
            defaults.reopen_on_monitor_return,
        )?,
        controller_only_start: launch_on_start
            && get_section_bool(
                ini,
                CONTROLLER_SECTION,
                "controller_only_start",
                defaults.controller_only_start,
            )?,
        open_on_os_startup: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "open_on_os_startup",
            defaults.open_on_os_startup,
        )?,
        ba1_enabled: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "ba1_enabled",
            defaults.ba1_enabled,
        )?,
        ba2_enabled: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "ba2_enabled",
            defaults.ba2_enabled,
        )?,
        keep_controller_alive_when_minimized: get_section_bool(
            ini,
            CONTROLLER_SECTION,
            "keep_controller_alive_when_minimized",
            defaults.keep_controller_alive_when_minimized,
        )?,
        launcher_x_action: optional_section(ini, CONTROLLER_SECTION, "launcher_x_action")
            .map(|value| value.parse::<LauncherXAction>())
            .transpose()?
            .unwrap_or(defaults.launcher_x_action),
        monitor_adapter: optional_section_u32(ini, CONTROLLER_SECTION, "monitor_adapter")?,
        background_image: optional_section(ini, CONTROLLER_SECTION, "background_image")
            .filter(|value| !value.trim().is_empty())
            .map(std::path::PathBuf::from),
        launcher_layout: optional_section(ini, CONTROLLER_SECTION, "launcher_layout")
            .unwrap_or(defaults.launcher_layout),
        orientation,
        calibration,
        controller,
    };
    crate::game_launcher_screen::validate_screen_config(&value)?;
    Ok(value)
}

fn parse_calibration(
    ini: &Ini,
    default_profile: OrientationProfile,
) -> Result<Option<MonitorCalibration>, String> {
    let monitor_id = optional_section(ini, CONTROLLER_SECTION, "calibration_monitor_id");
    let width = optional_section_u32(ini, CONTROLLER_SECTION, "calibration_mode_width")?;
    let height = optional_section_u32(ini, CONTROLLER_SECTION, "calibration_mode_height")?;
    let rotation = optional_section_u32(ini, CONTROLLER_SECTION, "calibration_rotation")?;
    let diameter = optional_section_u32(ini, CONTROLLER_SECTION, "calibration_diameter_px")?;
    let any = monitor_id.is_some()
        || width.is_some()
        || height.is_some()
        || rotation.is_some()
        || diameter.is_some();
    if !any {
        return Ok(None);
    }
    let required = |value: Option<u32>, key: &str| {
        value.ok_or_else(|| format!("incomplete [{CONTROLLER_SECTION}] calibration: missing {key}"))
    };
    let profile = optional_section(ini, CONTROLLER_SECTION, "calibration_profile")
        .map(|value| value.parse::<OrientationProfile>())
        .transpose()?
        .unwrap_or(default_profile);
    Ok(Some(MonitorCalibration {
        monitor_id: monitor_id.ok_or_else(|| {
            format!("incomplete [{CONTROLLER_SECTION}] calibration: missing calibration_monitor_id")
        })?,
        mode_width_px: required(width, "calibration_mode_width")?,
        mode_height_px: required(height, "calibration_mode_height")?,
        rotation_degrees: u16::try_from(required(rotation, "calibration_rotation")?)
            .map_err(|_| "calibration_rotation exceeds u16".to_string())?,
        profile,
        diameter_px: required(diameter, "calibration_diameter_px")?,
    }))
}

fn write_controller_screen(ini: &mut Ini, value: &ControllerScreenConfig) {
    let prior_option_keys: Vec<String> = ini
        .get_map_ref()
        .get(CONTROLLER_SECTION)
        .into_iter()
        .flat_map(|values| values.keys())
        .filter(|key| key.starts_with("controller_option."))
        .cloned()
        .collect();
    for key in prior_option_keys {
        ini.remove_key(CONTROLLER_SECTION, &key);
    }
    set_section_bool(ini, CONTROLLER_SECTION, "enabled", value.enabled);
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "launch_on_start",
        value.launch_on_start,
    );
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "reopen_after_game_close",
        value.reopen_after_game_close,
    );
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "reopen_on_monitor_return",
        value.reopen_on_monitor_return,
    );
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "controller_only_start",
        value.controller_only_start && value.launch_on_start,
    );
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "open_on_os_startup",
        value.open_on_os_startup,
    );
    set_section_bool(ini, CONTROLLER_SECTION, "ba1_enabled", value.ba1_enabled);
    set_section_bool(ini, CONTROLLER_SECTION, "ba2_enabled", value.ba2_enabled);
    set_section_bool(
        ini,
        CONTROLLER_SECTION,
        "keep_controller_alive_when_minimized",
        value.keep_controller_alive_when_minimized,
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "launcher_x_action",
        Some(&value.launcher_x_action.to_string()),
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "monitor_adapter",
        value
            .monitor_adapter
            .map(|adapter| adapter.to_string())
            .as_deref(),
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "background_image",
        value
            .background_image
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .as_deref(),
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "orientation",
        Some(&value.orientation.to_string()),
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "launcher_layout",
        Some(&value.launcher_layout),
    );
    ini.setstr(
        CONTROLLER_SECTION,
        "controller_driver",
        Some(&value.controller.driver_id),
    );
    for (key, option_value) in &value.controller.options {
        ini.setstr(
            CONTROLLER_SECTION,
            &format!("controller_option.{key}"),
            Some(option_value),
        );
    }

    const CALIBRATION_KEYS: [&str; 6] = [
        "calibration_monitor_id",
        "calibration_mode_width",
        "calibration_mode_height",
        "calibration_rotation",
        "calibration_profile",
        "calibration_diameter_px",
    ];
    if let Some(calibration) = &value.calibration {
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_monitor_id",
            Some(&calibration.monitor_id),
        );
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_mode_width",
            Some(&calibration.mode_width_px.to_string()),
        );
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_mode_height",
            Some(&calibration.mode_height_px.to_string()),
        );
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_rotation",
            Some(&calibration.rotation_degrees.to_string()),
        );
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_profile",
            Some(&calibration.profile.to_string()),
        );
        ini.setstr(
            CONTROLLER_SECTION,
            "calibration_diameter_px",
            Some(&calibration.diameter_px.to_string()),
        );
    } else {
        for key in CALIBRATION_KEYS {
            ini.remove_key(CONTROLLER_SECTION, key);
        }
    }
}

fn ini_parser() -> Ini {
    let mut defaults = IniDefault::default();
    defaults.enable_inline_comments = false;
    Ini::new_from_defaults(defaults)
}

fn optional_section(ini: &Ini, section: &str, key: &str) -> Option<String> {
    ini.get(section, key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_section_u32(ini: &Ini, section: &str, key: &str) -> Result<Option<u32>, String> {
    optional_section(ini, section, key)
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("invalid [{section}] {key}='{value}': expected u32"))
        })
        .transpose()
}

fn parse_launcher_hotkey(ini: &Ini) -> Result<Option<Hotkey>, String> {
    match ini.get(LAUNCHER_SECTION, "launcher_hotkey") {
        None => Ok(Some(default_launcher_hotkey())),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value
            .trim()
            .parse::<Hotkey>()
            .map(Some)
            .map_err(|error| format!("invalid [launcher] launcher_hotkey: {error}")),
    }
}

fn parse_close_game_hotkey(ini: &Ini) -> Result<Option<Hotkey>, String> {
    match ini.get(LAUNCHER_SECTION, "close_game_hotkey") {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value
            .trim()
            .parse::<Hotkey>()
            .map(Some)
            .map_err(|error| format!("invalid [launcher] close_game_hotkey: {error}")),
    }
}

const MAX_GAME_ID: u64 = i64::MAX as u64;

fn game_id_in_range(value: u64, what: &str) -> Result<u64, String> {
    if value > MAX_GAME_ID {
        return Err(format!(
            "{what} {value} is above the largest game id {MAX_GAME_ID}"
        ));
    }
    Ok(value)
}

fn optional_u64(ini: &Ini, key: &str) -> Result<Option<u64>, String> {
    match ini.get(LAUNCHER_SECTION, key) {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => value
            .trim()
            .parse::<u64>()
            .map(Some)
            .map_err(|_| format!("invalid [launcher] {key}='{value}': expected u64")),
    }
}

fn set_bool(ini: &mut Ini, key: &str, value: bool) {
    set_section_bool(ini, LAUNCHER_SECTION, key, value);
}

fn set_section_bool(ini: &mut Ini, section: &str, key: &str, value: bool) {
    ini.setstr(section, key, Some(if value { "true" } else { "false" }));
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/domain/config__tests.rs"]
mod tests;
