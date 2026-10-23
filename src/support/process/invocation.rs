use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::command_line::command_line;
use super::console_policy::ConsolePlan;
use super::image::is_script;
use super::interpreter::{comspec, interpreter_command_line, program_line};
use super::pause::paused_command_line;
use crate::support::win::wide;

pub struct Invocation {
    pub application: PathBuf,
    pub command_line: Vec<u16>,
}

pub fn invocation(
    program: &Path,
    arguments: &[OsString],
    plan: ConsolePlan,
) -> Result<Invocation, String> {
    if plan.pause_on_failure {
        let shell = comspec()?;
        let line = paused_command_line(&shell, program, arguments)?;
        return Ok(Invocation {
            application: shell,
            command_line: wide(&line),
        });
    }
    if is_script(program) {
        let shell = comspec()?;
        let line = interpreter_command_line(&shell, &program_line(program, arguments)?);
        return Ok(Invocation {
            application: shell,
            command_line: wide(&line),
        });
    }
    Ok(Invocation {
        application: program.to_path_buf(),
        command_line: command_line(program, arguments),
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/invocation__tests.rs"]
mod tests;
