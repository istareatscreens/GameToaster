use super::*;

#[test]
fn the_interpreter_is_the_configured_command_processor() {
    let shell = comspec();
    assert!(shell.is_file(), "{}", shell.display());
    assert!(shell
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("cmd.exe")));
}

#[test]
fn the_program_is_quoted_and_arguments_only_when_cmd_would_read_them() {
    let line = program_line(
        Path::new(r"C:\Games\a&b (x)\run.exe"),
        &[
            OsString::from("/d"),
            OsString::from("--gametoaster"),
            OsString::from(r"D:\State\request.json"),
            OsString::from("-8"),
            OsString::from("two words"),
            OsString::from("a^b!c;d,e=f"),
        ],
    )
    .unwrap();
    assert_eq!(
        line,
        r#""C:\Games\a&b (x)\run.exe" /d --gametoaster D:\State\request.json -8 "two words" "a^b!c;d,e=f""#
    );
}

#[test]
fn every_percent_is_escaped_so_no_two_pair_up() {
    let line = program_line(
        Path::new(r"C:\%USERNAME%\run.exe"),
        &[OsString::from(r"D:\100%\x%TEMP%y\request.json")],
    )
    .unwrap();
    assert_eq!(
        line,
        r#""C:\%%cd:~,%USERNAME%%cd:~,%\run.exe" "D:\100%%cd:~,%\x%%cd:~,%TEMP%%cd:~,%y\request.json""#
    );
}

#[test]
fn empty_arguments_and_trailing_backslashes_survive_the_c_runtime() {
    let line = program_line(
        Path::new(r"C:\Games\run.exe"),
        &[OsString::new(), OsString::from(r"C:\folder\")],
    )
    .unwrap();
    assert_eq!(line, r#""C:\Games\run.exe" "" "C:\folder\\""#);
}

#[test]
fn a_verbatim_program_path_is_written_the_way_cmd_reads_paths() {
    assert_eq!(
        program_line(Path::new(r"\\?\C:\Games\run.exe"), &[]).unwrap(),
        r#""C:\Games\run.exe""#
    );
    assert_eq!(
        program_line(Path::new(r"\\?\UNC\server\share\run.exe"), &[]).unwrap(),
        r#""\\server\share\run.exe""#
    );
    assert!(
        program_line(Path::new(r"\\?\C:\Games\run.exe."), &[]).is_err(),
        "a trailing dot names another file once the prefix is gone"
    );
}

#[test]
fn quotes_line_breaks_nuls_and_folder_paths_are_refused() {
    let program = Path::new(r"C:\Games\run.exe");
    for argument in ["say \"hi\"", "two\nlines", "carriage\rreturn", "nul\0byte"] {
        assert!(
            program_line(program, &[OsString::from(argument)]).is_err(),
            "{argument:?}"
        );
    }
    assert!(program_line(Path::new(r"C:\Games\"), &[]).is_err());
}

#[test]
fn the_interpreter_runs_the_line_between_its_outer_quotes_with_fixed_switches() {
    assert_eq!(
        interpreter_command_line(
            Path::new(r"C:\Windows\System32\cmd.exe"),
            r#""C:\Games\run.bat""#
        ),
        r#"C:\Windows\System32\cmd.exe /e:on /v:off /d /s /c ""C:\Games\run.bat"""#
    );
    assert_eq!(
        interpreter_command_line(Path::new(r"C:\Program Files\cmd.exe"), "x"),
        r#""C:\Program Files\cmd.exe" /e:on /v:off /d /s /c "x""#
    );
}
