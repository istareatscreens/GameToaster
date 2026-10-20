use std::collections::BTreeSet;
use std::time::{Duration, UNIX_EPOCH};

use super::*;

fn file_name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn a_backup_holds_the_file_under_its_name_what_it_was_and_when() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("GameToaster.ini");
    fs::write(&path, b"[launcher]\r\nschema_version=2\r\n").unwrap();
    let time = UNIX_EPOCH + Duration::from_millis(1_704_067_200_123);

    let labelled = keep_at(&path, Some("schema-2"), time).unwrap();
    let plain = keep_at(&path, None, time).unwrap();

    assert_eq!(labelled.parent(), Some(folder.path()));
    assert_eq!(
        file_name(&labelled),
        "GameToaster.ini.schema-2.20240101T000000.123Z.bak"
    );
    assert_eq!(
        file_name(&plain),
        "GameToaster.ini.20240101T000000.123Z.bak"
    );
    for copy in [&labelled, &plain] {
        assert_eq!(fs::read(copy).unwrap(), fs::read(&path).unwrap());
    }
}

#[test]
fn a_backup_in_the_same_millisecond_takes_the_next_free_name() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("GameToaster.ini");
    fs::write(&path, b"first").unwrap();
    let time = UNIX_EPOCH + Duration::from_millis(1_704_067_200_123);

    let first = keep_at(&path, None, time).unwrap();
    fs::write(&path, b"second").unwrap();
    let second = keep_at(&path, None, time).unwrap();

    assert_eq!(
        file_name(&first),
        "GameToaster.ini.20240101T000000.123Z.bak"
    );
    assert_eq!(
        file_name(&second),
        "GameToaster.ini.20240101T000000.123Z.1.bak"
    );
    assert_eq!(fs::read(&first).unwrap(), b"first");
    assert_eq!(fs::read(&second).unwrap(), b"second");
}

#[test]
fn backups_taken_now_never_replace_each_other() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("GameToaster.ini");
    fs::write(&path, b"original").unwrap();
    let copies: Vec<PathBuf> = (0..5).map(|_| keep(&path, None).unwrap()).collect();
    assert_eq!(copies.iter().collect::<BTreeSet<_>>().len(), copies.len());
    for copy in &copies {
        assert_eq!(fs::read(copy).unwrap(), b"original");
    }
}

#[test]
fn a_file_that_cannot_be_read_is_not_backed_up() {
    let folder = tempfile::tempdir().unwrap();
    let error = keep(&folder.path().join("missing.ini"), None).unwrap_err();
    assert!(error.starts_with("failed to back up"), "{error}");
    assert!(fs::read_dir(folder.path()).unwrap().next().is_none());
}
