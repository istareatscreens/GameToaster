use std::ffi::OsString;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::command_line::quote_argument;
use crate::support::system_folders;

const SWITCHES: &str = "/e:on /v:off /d /s /c";
const PERCENT: &str = "%%cd:~,%";

const UNQUOTED: &str = r"#$*+-./:?@\_";

pub fn comspec() -> Result<PathBuf, String> {
    let configured = std::env::var_os("ComSpec")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .map(PathBuf::into_os_string);
    system_folders::command_interpreter_from(configured, std::env::var_os("SystemRoot"))
}

pub fn interpreter_command_line(shell: &Path, line: &str) -> String {
    format!(
        "{} {}",
        quote_argument(shell.as_os_str()),
        interpreter_arguments(line)
    )
}

fn interpreter_arguments(line: &str) -> String {
    format!("{SWITCHES} \"{line}\"")
}

pub fn script_command(script: &Path, arguments: &[OsString]) -> Result<Command, String> {
    let mut command = Command::new(comspec()?);
    command.raw_arg(interpreter_arguments(&program_line(script, arguments)?));
    Ok(command)
}

pub fn program_line(program: &Path, arguments: &[OsString]) -> Result<String, String> {
    let program_text = user_path(program)?;
    let program_text = checked(&program_text)?;
    if program_text.ends_with('\\') {
        return Err(format!("{program_text:?} names no program"));
    }
    let mut line = format!("\"{}\"", program_text.replace('%', PERCENT));
    for argument in arguments {
        line.push(' ');
        line.push_str(&argument_token(checked(&argument.to_string_lossy())?));
    }
    Ok(line)
}

fn user_path(path: &Path) -> Result<String, String> {
    let text = path.as_os_str().to_string_lossy();
    let plain = if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        return Ok(text.into_owned());
    };
    match std::path::absolute(&plain) {
        Ok(full) if full.as_os_str() == plain.as_str() => Ok(plain),
        _ => Err(format!(
            "{} cannot be run by the command interpreter",
            path.display()
        )),
    }
}

fn checked(text: &str) -> Result<&str, String> {
    if text.contains(['"', '\r', '\n', '\0']) {
        return Err(format!(
            "a command line for the command interpreter cannot carry {text:?}"
        ));
    }
    Ok(text)
}

fn argument_token(argument: &str) -> String {
    let quote = argument.is_empty()
        || argument.ends_with('\\')
        || argument.chars().any(|character| {
            character.is_control()
                || (character.is_ascii()
                    && !character.is_ascii_alphanumeric()
                    && !UNQUOTED.contains(character))
        });
    let escaped = argument.replace('%', PERCENT);
    if !quote {
        return escaped;
    }
    let trailing = escaped.len() - escaped.trim_end_matches('\\').len();
    format!("\"{escaped}{}\"", "\\".repeat(trailing))
}

#[cfg(test)]
#[path = "../../../tests/unit/support/process/interpreter__tests.rs"]
mod tests;
