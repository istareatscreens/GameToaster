use super::*;

#[test]
fn an_extension_matches_in_any_letter_case_and_only_as_the_extension() {
    assert!(has_extension(Path::new(r"C:\g\game.EXE"), "exe"));
    assert!(has_extension(Path::new(r"C:\g\run.Bat"), "bat"));
    assert!(!has_extension(Path::new(r"C:\g\exe"), "exe"));
    assert!(!has_extension(Path::new(r"C:\g\game.exe.txt"), "exe"));
}

#[test]
fn letter_case_and_separators_do_not_distinguish_a_path() {
    assert!(paths_equal(
        Path::new(r"D:\Games\Arcade\game.exe"),
        Path::new("d:/games/ARCADE/Game.EXE")
    ));
    assert!(paths_equal(Path::new(r"C:\a\b"), Path::new("C:/a/b")));
    assert!(!paths_equal(
        Path::new(r"D:\Games\game.exe"),
        Path::new(r"D:\Games\game2.exe")
    ));
    assert!(!paths_equal(
        Path::new("game.exe"),
        Path::new(r"D:\game.exe")
    ));
}

#[test]
fn the_key_folds_case_and_separators_and_keeps_everything_else() {
    assert_eq!(
        path_key(Path::new(r"D:/Games\Arcade/Game.EXE")),
        r"d:\games\arcade\game.exe"
    );
    assert_eq!(
        path_key(Path::new("Ünïcode/É.exe")),
        r"Ünïcode\É.exe".to_ascii_lowercase()
    );
    assert_ne!(
        path_key(Path::new(r"C:\a\b")),
        path_key(Path::new(r"C:\a\b\"))
    );
}

#[test]
fn a_verbatim_prefix_does_not_make_another_file() {
    assert!(same_file(
        Path::new(r"\\?\C:\Games\Plugin\run.exe"),
        Path::new(r"c:\games\plugin\RUN.EXE")
    ));
    assert!(same_file(
        Path::new(r"\\?\UNC\server\share\run.exe"),
        Path::new(r"\\server\share\run.exe")
    ));
    assert!(!same_file(
        Path::new(r"C:\Games\Plugin\run.exe"),
        Path::new(r"C:\Games\Plugin\other.exe")
    ));
}

#[test]
fn a_junction_and_its_target_name_one_file_though_their_spellings_differ() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("run.exe"), b"MZ").unwrap();
    std::fs::write(target.join("other.exe"), b"MZ").unwrap();
    let junction = root.path().join("junction");
    let made = std::process::Command::new(crate::support::process::interpreter::comspec())
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(made.success());

    let through = junction.join("run.exe");
    assert!(!paths_equal(&through, &target.join("run.exe")));
    assert!(same_file(&through, &target.join("run.exe")));
    assert!(!same_file(&through, &target.join("other.exe")));
}
