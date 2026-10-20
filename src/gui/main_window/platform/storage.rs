use std::path::{Path, PathBuf};

use windows::Storage::ApplicationData;

use super::package::InstallKind;

pub fn state_root(kind: InstallKind) -> Result<PathBuf, String> {
    match kind {
        InstallKind::Portable => executable_folder(),
        InstallKind::Packaged => package_local_folder(),
    }
}

fn executable_folder() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current_exe(): {error}"))?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "current_exe() has no parent directory".to_string())
}

fn package_local_folder() -> Result<PathBuf, String> {
    let folder = ApplicationData::Current()
        .and_then(|data| data.LocalFolder())
        .map_err(|error| format!("failed to open the package local data folder: {error}"))?;
    let path = folder
        .Path()
        .map_err(|error| format!("failed to read the package local data folder path: {error}"))?;
    Ok(PathBuf::from(path.to_os_string()))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/platform/storage__tests.rs"]
mod tests;
