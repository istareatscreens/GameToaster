use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::support::paths::has_extension;

pub const MANIFEST_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyType {
    FindAndReplace,
    FindAndReplaceInDirectory,
    GameFolder,
    Absolute,
}

impl fmt::Display for CopyType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FindAndReplace => "find_and_replace",
            Self::FindAndReplaceInDirectory => "find_and_replace_in_directory",
            Self::GameFolder => "game_folder",
            Self::Absolute => "absolute",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchType {
    GameDirectory,
    FindInDirectory,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Prepare,
    Launch,
    Restore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Runtime {
    pub executable: String,
    pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConfigFile {
    pub config_file_location: String,
    pub config_file_copy_type: CopyType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_file_replace_path: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub config_add_to_config_menu: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PluginFile {
    pub plugin_file_location: String,
    pub plugin_file_copy_type: CopyType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_file_replace_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InstallScript {
    pub install_script_location: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub install_script_arguments: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub manifest_version: u32,
    pub plugin_id: String,
    pub plugin_version: Version,
    pub label: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_patcher: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_launcher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_watch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_watch_type: Option<WatchType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_files: Vec<ConfigFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugin_files: Vec<PluginFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub install_scripts: Vec<InstallScript>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if contains_numbered_keys(raw) {
            return Err(
                "numbered manifest keys are unsupported; use [[configFiles]] and [[pluginFiles]]"
                    .into(),
            );
        }
        let manifest: Self =
            toml::from_str(raw).map_err(|error| format!("invalid manifest.toml: {error}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn canonical_toml(&self) -> Result<String, String> {
        self.validate()?;
        let mut output = toml::to_string_pretty(self)
            .map_err(|error| format!("manifest encode failed: {error}"))?;
        if !output.ends_with('\n') {
            output.push('\n');
        }
        Ok(output.replace("\r\n", "\n"))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(format!(
                "manifestVersion {} is unsupported; expected {}",
                self.manifest_version, MANIFEST_VERSION
            ));
        }
        validate_plugin_id(&self.plugin_id)?;
        validate_text("label", &self.label, 1, 128)?;
        validate_text("description", &self.description, 1, 4096)?;

        let mut source_names = BTreeSet::new();
        for (index, entry) in self.config_files.iter().enumerate() {
            let field = format!("configFiles[{index}].configFileLocation");
            let normalized = normalize_package_path(&entry.config_file_location)
                .map_err(|error| format!("{field}: {error}"))?;
            source_names.insert(normalized.to_ascii_lowercase());
            validate_replace_path(
                &format!("configFiles[{index}]"),
                entry.config_file_copy_type,
                entry.config_file_replace_path.as_deref(),
            )?;
        }
        for (index, entry) in self.plugin_files.iter().enumerate() {
            let field = format!("pluginFiles[{index}].pluginFileLocation");
            let normalized = normalize_package_path(&entry.plugin_file_location)
                .map_err(|error| format!("{field}: {error}"))?;
            if !source_names.insert(normalized.to_ascii_lowercase()) {
                return Err(format!("{field}: duplicate package source '{normalized}'"));
            }
            validate_replace_path(
                &format!("pluginFiles[{index}]"),
                entry.plugin_file_copy_type,
                entry.plugin_file_replace_path.as_deref(),
            )?;
        }
        for (index, entry) in self.install_scripts.iter().enumerate() {
            let field = format!("installScripts[{index}].installScriptLocation");
            let normalized = normalize_package_path(&entry.install_script_location)
                .map_err(|error| format!("{field}: {error}"))?;
            if !source_names.insert(normalized.to_ascii_lowercase()) {
                return Err(format!("{field}: duplicate package source '{normalized}'"));
            }
            let extension = Path::new(&normalized)
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if !matches!(
                extension.to_ascii_lowercase().as_str(),
                "bat" | "cmd" | "ps1" | "exe"
            ) {
                return Err(format!(
                    "{field}: install scripts must use .bat, .cmd, .ps1, or .exe"
                ));
            }
            if matches!(extension.to_ascii_lowercase().as_str(), "bat" | "cmd")
                && (normalized.contains(['&', '|', '<', '>', '^', '%', '!', '(', ')', '"'])
                    || entry.install_script_arguments.iter().any(|argument| {
                        argument.contains(['&', '|', '<', '>', '^', '%', '!', '(', ')', '"'])
                    }))
            {
                return Err(format!(
                    "installScripts[{index}]: batch paths and arguments contain unsafe command characters"
                ));
            }
            if entry.install_script_arguments.len() > 64 {
                return Err(format!(
                    "installScripts[{index}].installScriptArguments has more than 64 entries"
                ));
            }
            for (argument_index, argument) in entry.install_script_arguments.iter().enumerate() {
                if argument.len() > 4096
                    || argument
                        .chars()
                        .any(|character| character == '\0' || character.is_control())
                {
                    return Err(format!(
                        "installScripts[{index}].installScriptArguments[{argument_index}] is invalid"
                    ));
                }
            }
        }

        if let Some(launcher) = &self.game_launcher {
            let launcher = normalize_package_path(launcher)
                .map_err(|error| format!("gameLauncher: {error}"))?;
            let matches = self
                .plugin_files
                .iter()
                .filter_map(|entry| normalize_package_path(&entry.plugin_file_location).ok())
                .filter(|source| source.eq_ignore_ascii_case(&launcher))
                .count();
            if matches != 1 {
                return Err(format!(
                    "gameLauncher must name exactly one declared plugin file; found {matches}"
                ));
            }
        }

        match (
            self.executable_watch.as_deref().map(str::trim),
            self.executable_watch_type,
        ) {
            (Some(""), Some(WatchType::FindInDirectory)) => {
                return Err(
                    "executableWatch is required for executableWatchType=find_in_directory".into(),
                )
            }
            (Some(value), _) if !value.is_empty() => {
                validate_basename_or_package_path("executableWatch", value)?;
            }
            (None, Some(WatchType::FindInDirectory)) => {
                return Err(
                    "executableWatch is required for executableWatchType=find_in_directory".into(),
                )
            }
            _ => {}
        }

        if self.is_patcher {
            let runtime = self
                .runtime
                .as_ref()
                .ok_or_else(|| "runtime is required when isPatcher=true".to_string())?;
            let executable = normalize_package_path(&runtime.executable)
                .map_err(|error| format!("runtime.executable: {error}"))?;
            if !source_names.contains(&executable.to_ascii_lowercase()) {
                return Err("runtime.executable must name a declared package file".into());
            }

            if !has_extension(Path::new(&executable), "exe") {
                return Err("runtime.executable must name an .exe program".into());
            }
            let operations: BTreeSet<_> = runtime.operations.iter().copied().collect();
            if operations.len() != runtime.operations.len() {
                return Err("runtime.operations contains duplicate values".into());
            }
            for required in [Operation::Prepare, Operation::Restore] {
                if !operations.contains(&required) {
                    return Err(format!(
                        "runtime.operations must include {} when isPatcher=true",
                        operation_name(required)
                    ));
                }
            }
            if self.game_launcher.as_deref().is_some_and(|launcher| {
                normalize_package_path(launcher)
                    .is_ok_and(|launcher| launcher.eq_ignore_ascii_case(&executable))
            }) && !operations.contains(&Operation::Launch)
            {
                return Err(
                    "runtime.operations must include launch when gameLauncher is the runtime"
                        .into(),
                );
            }
        } else if self.runtime.is_some() {
            return Err("runtime is only valid when isPatcher=true".into());
        }
        Ok(())
    }

    pub fn referenced_sources(&self) -> Result<Vec<String>, String> {
        self.validate()?;
        let mut paths = BTreeSet::new();
        for source in self
            .config_files
            .iter()
            .map(|entry| entry.config_file_location.as_str())
            .chain(
                self.plugin_files
                    .iter()
                    .map(|entry| entry.plugin_file_location.as_str())
                    .chain(
                        self.install_scripts
                            .iter()
                            .map(|entry| entry.install_script_location.as_str()),
                    ),
            )
        {
            paths.insert(normalize_package_path(source)?);
        }
        Ok(paths.into_iter().collect())
    }
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Prepare => "prepare",
        Operation::Launch => "launch",
        Operation::Restore => "restore",
    }
}

fn validate_plugin_id(value: &str) -> Result<(), String> {
    if !(1..=128).contains(&value.len())
        || value.starts_with('.')
        || value.ends_with('.')
        || value.contains("..")
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
    {
        return Err(
            "pluginId must be 1-128 lowercase ASCII letters, digits, '.', '-' or '_'".into(),
        );
    }
    Ok(())
}

fn validate_text(field: &str, value: &str, minimum: usize, maximum: usize) -> Result<(), String> {
    let length = value.trim().chars().count();
    if !(minimum..=maximum).contains(&length) {
        return Err(format!(
            "{field} must contain between {minimum} and {maximum} characters"
        ));
    }
    Ok(())
}

fn validate_replace_path(
    field: &str,
    copy_type: CopyType,
    replace_path: Option<&str>,
) -> Result<(), String> {
    let supplied = replace_path
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match copy_type {
        CopyType::GameFolder | CopyType::FindAndReplace if supplied.is_some() => Err(format!(
            "{field}: replace path is not allowed for copy type {copy_type}"
        )),
        CopyType::FindAndReplaceInDirectory | CopyType::Absolute if supplied.is_none() => Err(
            format!("{field}: replace path is required for copy type {copy_type}"),
        ),
        _ => {
            if let Some(path) = supplied {
                validate_destination_syntax(path)
                    .map_err(|error| format!("{field}: replace path: {error}"))?;
            }
            Ok(())
        }
    }
}

pub fn normalize_package_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("path must not be empty".into());
    }
    if trimmed.starts_with("\\\\")
        || trimmed.starts_with("//")
        || trimmed.starts_with(r"\\?\")
        || trimmed.starts_with(r"\\.\")
        || trimmed.as_bytes().get(1) == Some(&b':')
    {
        return Err("absolute, UNC, drive, and device paths are forbidden".into());
    }
    let replaced = trimmed.replace('\\', "/");
    let without_dot = replaced.strip_prefix("./").unwrap_or(&replaced);
    let mut parts = Vec::new();
    for part in without_dot.split('/') {
        validate_component(part, false)?;
        if part == "." {
            continue;
        }
        if part == ".." {
            return Err("'..' is forbidden in package source paths".into());
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return Err("path must name a file".into());
    }
    Ok(parts.join("/"))
}

pub fn package_path(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let normalized = normalize_package_path(raw)?;
    Ok(normalized
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part)))
}

pub fn validate_destination_syntax(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("\\\\")
        || trimmed.starts_with("//")
        || trimmed.starts_with(r"\\?\")
        || trimmed.starts_with(r"\\.\")
        || trimmed.as_bytes().get(1) == Some(&b':')
    {
        return Err("destination must be relative to the selected game folder".into());
    }
    let normalized = trimmed.replace('\\', "/");

    let components = normalized.strip_suffix('/').unwrap_or(&normalized);
    if components.is_empty() || components.ends_with('/') {
        return Err("empty path components are forbidden".into());
    }
    for component in components.split('/') {
        validate_component(component, true)?;
    }
    Ok(())
}

fn validate_basename_or_package_path(field: &str, raw: &str) -> Result<(), String> {
    normalize_package_path(raw)
        .map(|_| ())
        .map_err(|error| format!("{field}: {error}"))
}

fn validate_component(component: &str, allow_parent: bool) -> Result<(), String> {
    if component.is_empty() {
        return Err("empty path components are forbidden".into());
    }
    if component == "." {
        return Ok(());
    }
    if component == ".." {
        return if allow_parent {
            Ok(())
        } else {
            Err("'..' is forbidden".into())
        };
    }
    if component.contains(':') {
        return Err("alternate data stream ':' components are forbidden".into());
    }
    if component
        .chars()
        .any(|character| character == '\0' || character.is_control())
    {
        return Err("control characters are forbidden".into());
    }
    let base = component
        .trim_end_matches(['.', ' '])
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    let reserved = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'));
    if reserved || component.ends_with('.') || component.ends_with(' ') {
        return Err(format!("reserved Windows path component '{component}'"));
    }
    Ok(())
}

pub fn validate_package_file(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let path = package_path(root, raw)?;
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|error| format!("source {} cannot be read: {error}", path.display()))?;
    if !metadata.file_type().is_file() || is_reparse(&metadata) {
        return Err(format!(
            "source must be a regular non-reparse file: {}",
            path.display()
        ));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("package root cannot be resolved: {error}"))?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("source cannot be resolved: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!("source escapes package root: {}", path.display()));
    }
    Ok(path)
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn contains_numbered_keys(raw: &str) -> bool {
    raw.lines().any(|line| {
        let key = line
            .split_once('=')
            .map(|(key, _)| key.trim())
            .unwrap_or_default();
        [
            "configFileLocation",
            "configFileCopyType",
            "configFileReplacePath",
            "configAddToConfigMenu",
            "pluginFileLocation",
            "pluginFileCopyType",
            "pluginFileReplacePath",
            "installScriptLocation",
            "installScriptArguments",
        ]
        .iter()
        .any(|prefix| {
            key.strip_prefix(prefix).is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
            })
        })
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/manifest/mod__tests.rs"]
mod tests;
