use rust_i18n::t;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub fn open_config_target(target: &Path) -> Result<(), String> {
    use crate::support::paths::has_extension;
    use crate::support::process::image::is_script;

    if crate::launcher_gui::store_apps::is_store_target(target) {
        return crate::launcher_gui::store_apps::launch(target);
    }
    if is_script(target) || has_extension(target, "exe") {
        let mut command = if is_script(target) {
            let mut command = Command::new(crate::support::process::interpreter::comspec());
            command.raw_arg(format!("/S /C \"\"{}\"\"", target.display()));
            command
        } else {
            Command::new(target)
        };
        if let Some(directory) = target.parent() {
            command.current_dir(directory);
        }
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to run {}: {error}", target.display()))
    } else {
        Command::new("notepad.exe")
            .arg(target)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open {} in Notepad: {error}", target.display()))
    }
}

pub fn reveal_in_explorer(target: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", target.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open Explorer for {}: {error}", target.display()))
}

pub fn validate_selected_executable(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(
            t!("errors.launchable_not_absolute", path => path.display().to_string()).to_string(),
        );
    }
    if !path.is_file() {
        return Err(
            t!("errors.launchable_missing", path => path.display().to_string()).to_string(),
        );
    }
    if !crate::launcher_gui::model::is_launchable_extension(path) {
        return Err(t!("errors.launchable_type", path => path.display().to_string()).to_string());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/services/file_actions__tests.rs"]
mod tests;
