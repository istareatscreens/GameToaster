use super::*;

#[test]
fn backup_never_overwrites_same_tick_names() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("config.ini");
    fs::write(&source, b"original").unwrap();
    let (first, first_hash) = create_dated_backup(&source).unwrap();
    let (second, second_hash) = create_dated_backup(&source).unwrap();
    assert_ne!(first, second);
    assert_eq!(first_hash, second_hash);
    assert_eq!(fs::read(first).unwrap(), b"original");
    assert_eq!(fs::read(second).unwrap(), b"original");
}
