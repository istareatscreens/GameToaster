use super::*;

fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "gametoaster-support-fs-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn an_atomic_write_creates_the_folder_replaces_the_file_and_leaves_no_temporary() {
    let dir = test_dir("atomic");
    let path = dir.join("nested").join("file.txt");
    atomic_write(&path, b"first").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"first");
    atomic_write(&path, b"second, longer").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"second, longer");
    let names: Vec<_> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["file.txt"], "no temporary file is left behind");

    assert!(atomic_write(Path::new(r"C:\"), b"x").is_err());
    assert!(atomic_write(&path.join("child.txt"), b"x").is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn random_hex_and_staging_folders_are_fresh_every_time() {
    let first = random_hex(8).unwrap();
    let second = random_hex(8).unwrap();
    assert_eq!(first.len(), 16);
    assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_ne!(first, second);

    let dir = test_dir("staging");
    let one = create_staging_directory(&dir, "stage").unwrap();
    let two = create_staging_directory(&dir, "stage").unwrap();
    assert_ne!(one, two);
    assert!(one.is_dir() && two.is_dir());
    assert!(one
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("stage-"));
    fs::remove_dir_all(dir).unwrap();
}
