use super::*;
use crate::support::process::console_policy::ConsoleWindow;

fn plan(window: ConsoleWindow, pause_on_failure: bool) -> ConsolePlan {
    ConsolePlan {
        window,
        pause_on_failure,
    }
}

fn run(program: &str, plan: ConsolePlan) -> Result<(PathBuf, String), String> {
    let invocation = invocation(Path::new(program), &[OsString::from("--fast")], plan)?;
    let line = &invocation.command_line;
    assert_eq!(line.last(), Some(&0), "the command line ends with a NUL");
    Ok((
        invocation.application,
        String::from_utf16(&line[..line.len() - 1]).unwrap(),
    ))
}

#[test]
fn a_program_without_pause_runs_as_itself() {
    let (application, line) =
        run(r"C:\Games\game.exe", plan(ConsoleWindow::Windowless, false)).unwrap();
    assert_eq!(application, PathBuf::from(r"C:\Games\game.exe"));
    assert_eq!(line, r"C:\Games\game.exe --fast");
}

#[test]
fn a_script_runs_in_the_interpreter_with_its_path_escaped() {
    let shell = comspec().unwrap();
    let (application, line) = run(
        r"C:\Games\%USERNAME%\run.bat",
        plan(ConsoleWindow::Hidden, false),
    )
    .unwrap();
    assert_eq!(application, shell);
    assert_eq!(
        line,
        interpreter_command_line(
            &shell,
            r#""C:\Games\%%cd:~,%USERNAME%%cd:~,%\run.bat" --fast"#
        )
    );
}

#[test]
fn a_pausing_program_or_script_runs_under_the_pause_wrapper() {
    let shell = comspec().unwrap();
    for program in [r"C:\Games\game.exe", r"C:\Games\run.cmd"] {
        let (application, line) = run(program, plan(ConsoleWindow::Shown, true)).unwrap();
        assert_eq!(application, shell);
        assert_eq!(
            line,
            paused_command_line(&shell, Path::new(program), &[OsString::from("--fast")]).unwrap()
        );
    }
}

#[test]
fn a_line_the_interpreter_cannot_carry_is_refused() {
    let refused = invocation(
        Path::new(r"C:\Games\run.bat"),
        &[OsString::from("say \"hi\"")],
        plan(ConsoleWindow::Hidden, false),
    );
    assert!(refused.is_err());
}
