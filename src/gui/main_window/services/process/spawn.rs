use windows::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, PROCESS_CREATION_FLAGS,
};

use crate::support::process::console_policy::ConsoleWindow;

use super::SpawnSpec;

pub(super) fn validate_spawn_spec(spec: &SpawnSpec) -> Result<(), String> {
    if !spec.executable.is_absolute() || !spec.executable.is_file() {
        return Err(format!(
            "spawn executable is missing or not absolute: {}",
            spec.executable.display()
        ));
    }
    if !spec.working_directory.is_absolute() || !spec.working_directory.is_dir() {
        return Err(format!(
            "spawn working directory is missing or not absolute: {}",
            spec.working_directory.display()
        ));
    }
    Ok(())
}

pub(super) fn creation_flags(window: ConsoleWindow) -> PROCESS_CREATION_FLAGS {
    CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | window.creation_flags()
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/services/process/spawn__tests.rs"]
mod tests;
