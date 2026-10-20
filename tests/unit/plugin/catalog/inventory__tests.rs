use super::*;
use crate::plugin::manifest::{ConfigFile, CopyType};
use semver::Version;

fn fixture(root: &Path, contents: &[u8]) {
    fs::create_dir_all(root.join("payload")).unwrap();
    fs::write(root.join("payload/config.ini"), contents).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: "org.example.identity".into(),
        plugin_version: Version::parse("1.0.0").unwrap(),
        label: "Identity".into(),
        description: "Digest fixture".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: vec![ConfigFile {
            config_file_location: "payload/config.ini".into(),
            config_file_copy_type: CopyType::GameFolder,
            config_file_replace_path: None,
            config_add_to_config_menu: false,
        }],
        plugin_files: Vec::new(),
        install_scripts: Vec::new(),
    };
    fs::write(
        root.join("manifest.toml"),
        manifest.canonical_toml().unwrap(),
    )
    .unwrap();
}

#[test]
fn canonical_digest_is_stable_and_one_byte_sensitive() {
    let first = tempfile::tempdir().unwrap();
    fixture(first.path(), b"abc");
    let a = CanonicalPackage::from_directory(first.path()).unwrap();
    let b = CanonicalPackage::from_directory(first.path()).unwrap();
    assert_eq!(a.digests, b.digests);
    fs::write(first.path().join("payload/config.ini"), b"abd").unwrap();
    let changed = CanonicalPackage::from_directory(first.path()).unwrap();
    assert_ne!(a.digests.package_md5, changed.digests.package_md5);
    assert_ne!(a.digests.package_sha256, changed.digests.package_sha256);
}

#[test]
fn undeclared_neighbors_do_not_change_identity() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path(), b"abc");
    let before = CanonicalPackage::from_directory(root.path()).unwrap();
    fs::write(root.path().join("unrelated.bin"), b"ignored").unwrap();
    let after = CanonicalPackage::from_directory(root.path()).unwrap();
    assert_eq!(before.digests, after.digests);
}
