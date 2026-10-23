use std::io;
use std::path::Path;

use rust_i18n::t;

use crate::launcher_gui::platform::package::InstallKind;

pub fn state_folder_message(kind: InstallKind, folder: &Path, error: &io::Error) -> Option<String> {
    if kind == InstallKind::Portable && error.kind() == io::ErrorKind::PermissionDenied {
        Some(t!("startup.state_folder_not_writable", folder => folder.display()).to_string())
    } else {
        None
    }
}

pub fn gui_failure_message(error: &str) -> String {
    let text = t!("errors.gui_failed", error => error).to_string();
    if names_opengl(error) {
        format!("{text}\n\n{}", t!("startup.graphics_hint"))
    } else {
        text
    }
}

fn names_opengl(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    ["opengl", "wgl", "egl"]
        .iter()
        .any(|name| lower.contains(name))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/support/startup_failure__tests.rs"]
mod tests;
