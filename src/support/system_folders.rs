use std::ffi::OsString;
use std::path::PathBuf;

pub(crate) fn system_root_from(value: Option<OsString>) -> Result<PathBuf, String> {
    match value {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(
            "the SystemRoot environment variable is not set; Windows sets it for every program"
                .to_string(),
        ),
    }
}

pub fn fonts_folders() -> Vec<PathBuf> {
    fonts_folders_from(
        std::env::var_os("SystemRoot"),
        std::env::var_os("LOCALAPPDATA"),
    )
}

pub(crate) fn fonts_folders_from(
    system_root: Option<OsString>,
    local_app_data: Option<OsString>,
) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Ok(root) = system_root_from(system_root) {
        folders.push(root.join("Fonts"));
    }
    if let Some(local) = local_app_data.filter(|value| !value.is_empty()) {
        folders.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    folders
}

pub(crate) fn command_interpreter_from(
    com_spec: Option<OsString>,
    system_root: Option<OsString>,
) -> Result<PathBuf, String> {
    if let Some(value) = com_spec.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }
    Ok(system_root_from(system_root)?
        .join("System32")
        .join("cmd.exe"))
}

pub fn windows_powershell() -> Result<PathBuf, String> {
    windows_powershell_from(std::env::var_os("SystemRoot"))
}

pub(crate) fn windows_powershell_from(system_root: Option<OsString>) -> Result<PathBuf, String> {
    Ok(system_root_from(system_root)?
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe"))
}

#[cfg(test)]
#[path = "../../tests/unit/support/system_folders__tests.rs"]
mod tests;
