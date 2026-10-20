use super::*;
use crate::plugin::manifest::{ConfigFile, CopyType, Manifest};

fn package(root: &Path, version: &str, bytes: &[u8]) {
    fs::create_dir_all(root.join("payload")).unwrap();
    fs::write(root.join("payload/config.ini"), bytes).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: "org.example.catalog".into(),
        plugin_version: Version::parse(version).unwrap(),
        label: "Catalog".into(),
        description: "Catalog test".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: vec![ConfigFile {
            config_file_location: "payload/config.ini".into(),
            config_file_copy_type: CopyType::GameFolder,
            config_file_replace_path: None,
            config_add_to_config_menu: true,
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
fn loose_import_is_immutable_idempotent_and_excludes_neighbors() {
    let state = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    package(source.path(), "1.0.0", b"one");
    fs::write(source.path().join("unrelated.bin"), b"no").unwrap();
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    let first = catalog
        .import_manifest(&source.path().join("manifest.toml"))
        .unwrap();
    let ImportOutcome::Imported(revision) = first else {
        panic!("first import was not new")
    };
    assert!(!revision.root.join("unrelated.bin").exists());
    assert!(matches!(
        catalog
            .import_manifest(&source.path().join("manifest.toml"))
            .unwrap(),
        ImportOutcome::AlreadyPresent(_)
    ));
    assert_eq!(catalog.list().unwrap().len(), 1);
}

fn import_later(catalog: &Catalog, source: &Path) -> CatalogRevision {
    std::thread::sleep(std::time::Duration::from_millis(2));
    match catalog
        .import_manifest(&source.join("manifest.toml"))
        .unwrap()
    {
        ImportOutcome::Imported(revision) | ImportOutcome::AlreadyPresent(revision) => revision,
    }
}

#[test]
fn a_rebuilt_package_of_a_held_version_is_imported_beside_it_and_is_the_latest() {
    let state = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    package(source.path(), "1.0.0", b"one");
    let first = import_later(&catalog, source.path());
    fs::write(source.path().join("payload/config.ini"), b"two").unwrap();
    let rebuilt = import_later(&catalog, source.path());
    assert_ne!(rebuilt.package_sha256, first.package_sha256);
    assert!(rebuilt.imported_at > first.imported_at);

    let listed: Vec<_> = catalog
        .list()
        .unwrap()
        .into_iter()
        .map(|revision| revision.package_sha256)
        .collect();
    assert_eq!(
        listed,
        [first.package_sha256.clone(), rebuilt.package_sha256.clone()]
    );
    assert_eq!(
        catalog
            .select_latest("org.example.catalog", false)
            .unwrap()
            .unwrap()
            .package_sha256,
        rebuilt.package_sha256
    );

    assert_eq!(
        catalog
            .select_exact(
                "org.example.catalog",
                &Version::parse("1.0.0").unwrap(),
                &first.package_sha256
            )
            .unwrap()
            .root,
        first.root
    );
    assert!(matches!(
        catalog
            .import_manifest(&source.path().join("manifest.toml"))
            .unwrap(),
        ImportOutcome::AlreadyPresent(revision) if revision.package_sha256 == rebuilt.package_sha256
    ));
    assert_eq!(catalog.list().unwrap().len(), 2);
}

fn set_import_time(revision: &CatalogRevision, value: Option<u64>) {
    let metadata = revision.root.join("catalog.toml");
    set_tree_readonly(&revision.root, false).unwrap();
    let mut lines: Vec<String> = fs::read_to_string(&metadata)
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("importedAt"))
        .map(str::to_owned)
        .collect();
    if let Some(value) = value {
        let first_table = lines
            .iter()
            .position(|line| line.starts_with("[["))
            .unwrap_or(lines.len());
        lines.insert(first_table, format!("importedAt = {value}"));
    }
    fs::write(
        &metadata,
        lines.join(
            "
",
        ) + "
",
    )
    .unwrap();
    set_tree_readonly(&revision.root, true).unwrap();
}

#[test]
fn an_import_time_is_later_than_every_recorded_import_of_the_plugin_whatever_the_clock_says() {
    let state = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    package(source.path(), "1.0.0", b"one");
    let first = import_later(&catalog, source.path());

    let ahead = first.imported_at + 100 * 365 * 24 * 60 * 60 * 1000;
    set_import_time(&first, Some(ahead));
    fs::write(source.path().join("payload/config.ini"), b"two").unwrap();
    let rebuilt = import_later(&catalog, source.path());
    assert_eq!(rebuilt.imported_at, ahead + 1);
    assert_eq!(
        catalog
            .select_latest("org.example.catalog", false)
            .unwrap()
            .unwrap()
            .package_sha256,
        rebuilt.package_sha256
    );
}

#[test]
fn a_revision_without_an_import_time_ranks_below_a_later_import_of_its_version() {
    let state = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    package(source.path(), "1.0.0", b"one");
    let older = import_later(&catalog, source.path());

    set_import_time(&older, None);
    assert_eq!(catalog.verify_revision(&older.root).unwrap().imported_at, 0);
    fs::write(source.path().join("payload/config.ini"), b"two").unwrap();
    let rebuilt = import_later(&catalog, source.path());
    assert_eq!(
        catalog
            .select_latest("org.example.catalog", false)
            .unwrap()
            .unwrap()
            .package_sha256,
        rebuilt.package_sha256
    );
}

#[test]
fn latest_ignores_prerelease_unless_explicit() {
    let state = tempfile::tempdir().unwrap();
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    for (version, bytes) in [
        ("1.0.0", b"a".as_slice()),
        ("2.0.0-beta.1", b"b".as_slice()),
        ("1.5.0", b"c".as_slice()),
    ] {
        let source = tempfile::tempdir().unwrap();
        package(source.path(), version, bytes);
        catalog
            .import_manifest(&source.path().join("manifest.toml"))
            .unwrap();
    }
    assert_eq!(
        catalog
            .select_latest("org.example.catalog", false)
            .unwrap()
            .unwrap()
            .plugin_version,
        Version::parse("1.5.0").unwrap()
    );
    assert_eq!(
        catalog
            .select_latest("org.example.catalog", true)
            .unwrap()
            .unwrap()
            .plugin_version,
        Version::parse("2.0.0-beta.1").unwrap()
    );
}

#[test]
fn removal_is_refused_when_assigned_and_recoverable_otherwise() {
    let state = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    package(source.path(), "1.0.0", b"one");
    let catalog = Catalog::open(state.path().join("plugins")).unwrap();
    let ImportOutcome::Imported(revision) = catalog
        .import_manifest(&source.path().join("manifest.toml"))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(catalog.remove_revision(&revision, &[7]).is_err());
    let moved = catalog.remove_revision(&revision, &[]).unwrap();
    assert!(moved.is_dir());
    assert!(!revision.root.exists());
}
