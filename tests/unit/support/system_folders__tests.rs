use super::*;
use std::path::Path;

fn set(value: &str) -> Option<OsString> {
    Some(OsString::from(value))
}

#[test]
fn the_windows_folder_comes_from_system_root_and_is_never_guessed() {
    assert_eq!(
        system_root_from(set(r"E:\Windows")).unwrap(),
        Path::new(r"E:\Windows")
    );
    let error = system_root_from(None).unwrap_err();
    assert!(error.contains("SystemRoot"), "{error}");
    assert!(system_root_from(set("")).is_err());

    assert!(system_root_from(std::env::var_os("SystemRoot")).is_ok());
}

#[test]
fn the_fonts_folders_are_the_system_folder_then_the_per_user_folder() {
    assert_eq!(
        fonts_folders_from(set(r"E:\Windows"), set(r"E:\Home\someone\AppData\Local")),
        vec![
            PathBuf::from(r"E:\Windows\Fonts"),
            PathBuf::from(r"E:\Home\someone\AppData\Local\Microsoft\Windows\Fonts"),
        ]
    );
    assert_eq!(
        fonts_folders_from(set(r"E:\Windows"), None),
        vec![PathBuf::from(r"E:\Windows\Fonts")]
    );
    assert_eq!(
        fonts_folders_from(None, set(r"E:\Local")),
        vec![PathBuf::from(r"E:\Local\Microsoft\Windows\Fonts")]
    );
    assert!(fonts_folders_from(None, set("")).is_empty());
    assert!(!fonts_folders().is_empty());
}

#[test]
fn the_command_interpreter_is_comspec_else_cmd_in_the_system_folder() {
    assert_eq!(
        command_interpreter_from(set(r"E:\Windows\System32\cmd.exe"), None).unwrap(),
        Path::new(r"E:\Windows\System32\cmd.exe")
    );
    assert_eq!(
        command_interpreter_from(None, set(r"E:\Windows")).unwrap(),
        Path::new(r"E:\Windows\System32\cmd.exe")
    );
    assert_eq!(
        command_interpreter_from(set(""), set(r"E:\Windows")).unwrap(),
        Path::new(r"E:\Windows\System32\cmd.exe")
    );
    let error = command_interpreter_from(None, None).unwrap_err();
    assert!(error.contains("SystemRoot"), "{error}");
}

#[test]
fn windows_powershell_lives_in_the_system_folder() {
    assert_eq!(
        windows_powershell_from(set(r"E:\Windows")).unwrap(),
        Path::new(r"E:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
    );
    assert!(windows_powershell_from(None).is_err());
    assert!(windows_powershell().unwrap().is_file());
}
