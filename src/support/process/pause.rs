use std::ffi::OsString;
use std::path::Path;

use super::interpreter::{interpreter_command_line, program_line};

const ON_FAILURE: &str = "(pause & call exit %^errorlevel%)";

pub fn paused_command_line(
    shell: &Path,
    program: &Path,
    arguments: &[OsString],
) -> Result<String, String> {
    let line = program_line(program, arguments)?;
    Ok(interpreter_command_line(
        shell,
        &format!("{line} & (if errorlevel 1 {ON_FAILURE} else if not errorlevel 0 {ON_FAILURE})"),
    ))
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/pause__tests.rs"]
mod tests;
