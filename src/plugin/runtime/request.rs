use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::support::fs::{atomic_write, random_hex};
use crate::support::hex;

pub const REQUEST_LIMIT: usize = 65_536;
pub const PROTOCOL: u32 = 2;
pub const SWITCH: &str = "--gametoaster";
pub const PIPE_VARIABLE: &str = "GAMETOASTER_PIPE";
pub const OPERATION_ID_VARIABLE: &str = "GAMETOASTER_OPERATION_ID";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Prepare,
    Launch,
    Restore,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Launch => "launch",
            Self::Restore => "restore",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequestContext {
    pub plugin_id: String,
    pub plugin_version: String,
    pub plugin_sha256: String,
    pub numeric_game_id: u64,
    pub assignment_id: String,
    pub game_folder: PathBuf,
    pub selected_target: PathBuf,
    pub resolved_launcher: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub protocol: u32,
    pub operation: Operation,
    pub operation_id: String,
    pub pipe: String,
    pub game: GameRequest,
    pub plugin: PluginRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRequest {
    pub id: String,
    pub folder: String,
    pub executable: String,
    pub launcher: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequest {
    pub id: String,
    pub version: String,
    pub package_sha256: String,
    pub assignment_id: String,
}

impl Request {
    pub fn new(
        operation: Operation,
        context: &RequestContext,
        operation_id: String,
        pipe: String,
    ) -> Result<Self, String> {
        let request = Self {
            protocol: PROTOCOL,
            operation,
            operation_id,
            pipe,
            game: GameRequest {
                id: protocol_game_id(context.numeric_game_id),
                folder: path_text(&context.game_folder)?,
                executable: path_text(&context.selected_target)?,
                launcher: context
                    .resolved_launcher
                    .as_deref()
                    .map(path_text)
                    .transpose()?,
            },
            plugin: PluginRequest {
                id: context.plugin_id.clone(),
                version: context.plugin_version.clone(),
                package_sha256: context.plugin_sha256.to_ascii_lowercase(),
                assignment_id: context.assignment_id.to_ascii_lowercase(),
            },
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.protocol != PROTOCOL {
            return Err(format!("runtime request must use protocol {PROTOCOL}"));
        }
        validate_hex("operationId", &self.operation_id, 32)?;
        if self.pipe != super::channel::pipe_name(&self.operation_id) {
            return Err("runtime request pipe does not match its operation id".into());
        }
        validate_hex("game.id", &self.game.id, 32)?;
        validate_absolute_path("game.folder", &self.game.folder)?;
        validate_absolute_path("game.executable", &self.game.executable)?;
        if let Some(launcher) = &self.game.launcher {
            validate_absolute_path("game.launcher", launcher)?;
        }
        if self.plugin.id.is_empty() || self.plugin.id.len() > 128 {
            return Err("plugin.id must be 1-128 bytes".into());
        }
        if self.plugin.version.is_empty()
            || self.plugin.version.len() > 64
            || semver::Version::parse(&self.plugin.version).is_err()
        {
            return Err("plugin.version is not valid bounded SemVer".into());
        }
        validate_hex("plugin.packageSha256", &self.plugin.package_sha256, 64)?;
        validate_hex("plugin.assignmentId", &self.plugin.assignment_id, 32)?;
        Ok(())
    }
}

pub struct RequestFile {
    pub request: Request,
    pub directory: PathBuf,
    pub path: PathBuf,
    argument_path: PathBuf,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl RequestFile {
    pub fn create(
        session_state_root: &Path,
        operation: Operation,
        context: &RequestContext,
        operation_id: &str,
        pipe: &str,
    ) -> Result<Self, String> {
        let request = Request::new(operation, context, operation_id.to_owned(), pipe.to_owned())?;
        let directory = session_state_root.join(operation_id);
        if directory.exists() {
            return Err(format!(
                "runtime operation directory collision: {}",
                directory.display()
            ));
        }
        fs::create_dir_all(&directory).map_err(|error| {
            format!(
                "failed to create runtime operation directory {}: {error}",
                directory.display()
            )
        })?;
        let path = directory.join("request.json");
        let argument_path = normalized_local_argument(&path)?;
        let stdout_path = directory.join("stdout.log");
        let stderr_path = directory.join("stderr.log");
        let bytes = serde_json::to_vec_pretty(&request)
            .map_err(|error| format!("failed to serialize runtime request: {error}"))?;
        if bytes.len() > REQUEST_LIMIT {
            return Err(format!(
                "runtime request exceeds {REQUEST_LIMIT} bytes: {}",
                bytes.len()
            ));
        }
        atomic_write(&path, &bytes)?;
        Ok(Self {
            request,
            directory,
            path,
            argument_path,
            stdout_path,
            stderr_path,
        })
    }

    pub fn capture_files(&self) -> Result<(File, File), String> {
        let create = |path: &Path| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))
        };
        let stdout = create(&self.stdout_path)?;
        let stderr = match create(&self.stderr_path) {
            Ok(file) => file,
            Err(error) => {
                drop(stdout);
                let _ = fs::remove_file(&self.stdout_path);
                return Err(error);
            }
        };
        Ok((stdout, stderr))
    }

    pub fn output_paths(&self) -> (&Path, &Path) {
        (&self.stdout_path, &self.stderr_path)
    }

    pub fn argv(&self) -> Vec<OsString> {
        vec![
            OsString::from(SWITCH),
            self.argument_path.as_os_str().to_os_string(),
        ]
    }

    pub fn environment(&self) -> Vec<(OsString, OsString)> {
        vec![
            (
                OsString::from(PIPE_VARIABLE),
                OsString::from(&self.request.pipe),
            ),
            (
                OsString::from(OPERATION_ID_VARIABLE),
                OsString::from(&self.request.operation_id),
            ),
        ]
    }
}

impl Drop for RequestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(&self.stdout_path);
        let _ = fs::remove_file(&self.stderr_path);
        let _ = fs::remove_dir(&self.directory);
    }
}

pub fn new_operation_id() -> Result<String, String> {
    random_hex(16)
}

pub fn protocol_game_id(numeric_id: u64) -> String {
    let digest = Sha256::digest(format!("GameToaster-game-id-v1:{numeric_id}").as_bytes());
    hex::encode(&digest[..16])
}

fn path_text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("runtime path is not valid Unicode: {}", path.display()))
}

fn normalized_local_argument(path: &Path) -> Result<PathBuf, String> {
    let text = path
        .to_str()
        .ok_or_else(|| format!("request path is not valid Unicode: {}", path.display()))?;
    let local = text.strip_prefix(r"\\?\").unwrap_or(text);
    if local
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("UNC\\"))
    {
        return Err("request path cannot use a UNC namespace".into());
    }
    validate_absolute_path("request path", local)?;
    Ok(PathBuf::from(local))
}

fn validate_hex(field: &str, value: &str, length: usize) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!(
            "{field} must be {length} lowercase hexadecimal bytes"
        ));
    }
    Ok(())
}

fn validate_absolute_path(field: &str, value: &str) -> Result<(), String> {
    if value.len() < 3 || value.len() > 32_767 {
        return Err(format!("{field} has an invalid length"));
    }
    let path = Path::new(value);
    if !path.is_absolute()
        || value.starts_with(r"\\")
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(format!("{field} must be a normalized local absolute path"));
    }
    let without_prefix = value.get(2..).unwrap_or_default();
    if without_prefix.contains(':') {
        return Err(format!("{field} contains an alternate data stream"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/runtime/request__tests.rs"]
mod tests;
