use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::*;
use crate::support::process::command_line::quote_argument;
use crate::support::process::interpreter::comspec;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const FOLDERS: &[&str] = &[
    "two words",
    "a&b",
    "c^d",
    "e(f)",
    "g!h!i",
    "j%k",
    "%USERNAME%",
    "x%TEMP%y",
    "semi;colon,comma=equals",
    "\u{30b2}\u{30fc}\u{30e0}",
];

fn run(command_line: &str) -> (i32, String) {
    let shell = comspec();
    let rest = command_line
        .strip_prefix(&format!("{} ", quote_argument(shell.as_os_str())))
        .expect("the line starts with the interpreter");
    let output = Command::new(&shell)
        .raw_arg(rest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .unwrap();
    let mut printed = String::from_utf8_lossy(&output.stdout).into_owned();
    printed.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap(), printed)
}

fn exit_program(folder: &Path) -> PathBuf {
    std::fs::create_dir_all(folder).unwrap();
    let program = folder.join("exit tool.exe");
    std::fs::copy(comspec(), &program).unwrap();
    program
}

#[test]
fn the_line_tests_the_error_level_then_pauses_and_exits_with_it() {
    assert_eq!(
        paused_command_line(
            Path::new(r"C:\Windows\System32\cmd.exe"),
            Path::new(r"C:\Games\run.exe"),
            &[OsString::from("--fast")]
        )
        .unwrap(),
        concat!(
            r#"C:\Windows\System32\cmd.exe /e:on /v:off /d /s /c ""C:\Games\run.exe" --fast & "#,
            r#"(if errorlevel 1 (pause & call exit %^errorlevel%) "#,
            r#"else if not errorlevel 0 (pause & call exit %^errorlevel%))""#
        )
    );
}

#[test]
fn a_program_pauses_on_every_failure_and_its_exact_code_comes_back() {
    let root = tempfile::tempdir().unwrap();
    for folder in FOLDERS {
        let program = exit_program(&root.path().join(folder));
        for code in [0, 20, -8, -1_073_741_819] {
            let line = paused_command_line(
                &comspec(),
                &program,
                &[
                    OsString::from("/d"),
                    OsString::from("/c"),
                    OsString::from("exit"),
                    OsString::from(code.to_string()),
                ],
            )
            .unwrap();
            let (exit, printed) = run(&line);
            assert_eq!(exit, code, "{folder}: {printed}");
            assert_eq!(
                printed.trim().is_empty(),
                code == 0,
                "{folder} {code}: only a failure pauses, and the pause prints: {printed:?}"
            );
        }
    }
}

#[test]
fn a_script_pauses_whenever_its_last_error_level_is_not_zero() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("a&b %USERNAME% (x)");
    std::fs::create_dir_all(&folder).unwrap();
    for (name, body, code) in [
        ("succeeds.bat", "@rem nothing to do\r\n", 0),
        ("fails.bat", "@exit /b 20\r\n", 20),
        ("negative.bat", "@exit /b -8\r\n", -8),
        (
            "fails then a quiet command.bat",
            "@dir \"%~dp0no such folder\" >nul 2>&1\r\n@rem the error level stays 1\r\n",
            1,
        ),
    ] {
        let script = folder.join(name);
        std::fs::write(&script, body).unwrap();
        let (exit, printed) = run(&paused_command_line(&comspec(), &script, &[]).unwrap());
        assert_eq!(exit, code, "{name}: {printed}");
        assert_eq!(printed.trim().is_empty(), code == 0, "{name}: {printed:?}");
    }
}
