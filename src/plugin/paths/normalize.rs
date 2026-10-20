use std::path::{Component, Path, PathBuf};

use crate::plugin::manifest::validate_destination_syntax;

pub(super) fn resolve_relative(base: &Path, raw: &str) -> Result<PathBuf, String> {
    validate_destination_syntax(raw)?;
    let relative = raw.replace('/', "\\");
    normalize_lexical(&base.join(relative))
}

pub(super) fn normalize_lexical(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("path is not absolute: {}", path.display()));
    }
    let mut prefix: Option<std::ffi::OsString> = None;
    let mut has_root = false;
    let mut parts = Vec::<std::ffi::OsString>::new();
    for component in path.components() {
        match component {
            Component::Prefix(value) => prefix = Some(value.as_os_str().to_os_string()),
            Component::RootDir => has_root = true,
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(format!(
                        "path traverses above its volume root: {}",
                        path.display()
                    ));
                }
            }
            Component::Normal(value) => parts.push(value.to_os_string()),
        }
    }
    let mut output = PathBuf::new();
    if let Some(prefix) = prefix {
        output.push(prefix);
    }
    if has_root {
        output.push("\\");
    }
    for part in parts {
        output.push(part);
    }
    Ok(output)
}

pub(super) fn ends_with_separator(raw: &str) -> bool {
    raw.ends_with('/') || raw.ends_with('\\')
}

pub(super) fn validate_sha256(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("plugin-version SHA-256 must contain exactly 64 hexadecimal characters".into());
    }
    Ok(())
}

pub(super) fn path_starts_with_ci(path: &Path, root: &Path) -> bool {
    let path_components: Vec<_> = path
        .components()
        .map(|value| value.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    let root_components: Vec<_> = root
        .components()
        .map(|value| value.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    root_components.len() <= path_components.len()
        && root_components
            .iter()
            .zip(path_components.iter())
            .all(|(left, right)| left == right)
}
