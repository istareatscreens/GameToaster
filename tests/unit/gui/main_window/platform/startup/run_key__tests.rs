use super::*;

#[test]
fn a_moved_executable_gets_its_entry_rewritten_and_the_same_file_does_not() {
    let current = Path::new(r"E:\new\GameToaster.exe");
    let gone = |_: &Path| false;
    let rewritten = Some(r#""E:\new\GameToaster.exe" --tray"#.to_string());
    assert_eq!(
        repaired_command(r#""D:\old\GameToaster.exe" --tray"#, current, gone),
        rewritten
    );

    assert_eq!(
        repaired_command(r"D:\old\GameToaster.exe --tray", current, gone),
        rewritten
    );

    assert_eq!(
        repaired_command(r#""e:\NEW\gametoaster.EXE" --tray"#, current, gone),
        None
    );
    assert_eq!(
        repaired_command(r#""E:\new\GameToaster.exe""#, current, gone),
        None
    );

    assert_eq!(repaired_command("", current, gone), None);
    assert_eq!(repaired_command(r#""""#, current, gone), None);
}

#[test]
fn another_copy_that_still_exists_keeps_its_entry() {
    let current = Path::new(r"E:\new\GameToaster.exe");
    let other = Path::new(r"D:\other\GameToaster.exe");
    let on_disk = |path: &Path| path == other;
    assert_eq!(
        repaired_command(r#""D:\other\GameToaster.exe" --tray"#, current, on_disk),
        None
    );
    assert_eq!(
        repaired_command(r#""D:\gone\GameToaster.exe" --tray"#, current, on_disk),
        Some(r#""E:\new\GameToaster.exe" --tray"#.to_string())
    );
}

#[test]
fn the_stored_executable_is_the_quoted_token_or_the_first_word() {
    assert_eq!(
        stored_executable(r#""C:\a b\GameToaster.exe" --tray"#),
        Some(r"C:\a b\GameToaster.exe")
    );
    assert_eq!(
        stored_executable(r"C:\ab\GameToaster.exe --tray"),
        Some(r"C:\ab\GameToaster.exe")
    );
    assert_eq!(stored_executable(r#""C:\unterminated --tray"#), None);
    assert_eq!(stored_executable("   "), None);
    assert_eq!(
        command_for(Path::new(r"C:\a b\GameToaster.exe")),
        r#""C:\a b\GameToaster.exe" --tray"#
    );
}

#[test]
fn the_tray_flag_marks_a_sign_in_start_in_any_letter_case() {
    let args = |values: &[&str]| -> Vec<OsString> { values.iter().map(OsString::from).collect() };
    assert!(launched_at_sign_in(&args(&["--tray"])));
    assert!(launched_at_sign_in(&args(&["--other", "--TRAY"])));
    assert!(!launched_at_sign_in(&args(&[])));
    assert!(!launched_at_sign_in(&args(&["--trayicon", "tray"])));
}
