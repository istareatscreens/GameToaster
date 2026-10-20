use super::*;

#[test]
fn selection_defaults_are_explicit_and_digest_is_short_display_only() {
    let descriptor = PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: "org.example.plugin".into(),
            plugin_version: Version::parse("1.2.3").unwrap(),
            package_md5: "a".repeat(32),
            package_sha256: "b".repeat(64),
            imported_at: 0,
            root: PathBuf::from(r"C:\catalog\revision"),
        },
        manifest: Manifest::parse(
            r#"
manifestVersion = 1
pluginId = "org.example.plugin"
pluginVersion = "1.2.3"
label = "Example"
description = "Example plugin"
"#,
        )
        .unwrap(),
    };
    let selection = PluginSelection::exact(&descriptor);
    assert!(selection.track_latest);
    assert!(!selection.copy_zero_matches_to_game_folder);
    assert_eq!(descriptor.digest_summary(), "bbbbbbbbbbbb");
}

#[test]
fn edited_revision_is_a_further_revision_beside_the_source_and_preserves_its_boundary() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("payload.bin"), b"payload").unwrap();
    fs::write(
        source.join("manifest.toml"),
        r#"
manifestVersion = 1
pluginId = "org.example.edit"
pluginVersion = "1.0.0"
label = "Edit"
description = "Editable plugin"

[[pluginFiles]]
pluginFileLocation = "payload.bin"
pluginFileCopyType = "game_folder"
"#,
    )
    .unwrap();
    let catalog = Catalog::open(root.path().join("catalog")).unwrap();
    let imported = catalog
        .import_manifest(&source.join("manifest.toml"))
        .unwrap();
    let revision = match imported {
        ImportOutcome::Imported(revision) | ImportOutcome::AlreadyPresent(revision) => revision,
    };
    let descriptor = list(&catalog)
        .unwrap()
        .into_iter()
        .find(|item| item.revision == revision)
        .unwrap();
    let same = descriptor.manifest.canonical_toml().unwrap();
    assert!(matches!(
        import_edited_revision(&catalog, &root.path().join("drafts"), &descriptor, &same,).unwrap(),
        ImportOutcome::AlreadyPresent(_)
    ));
    assert_eq!(list(&catalog).unwrap().len(), 1);

    let edited = same.replace("Editable plugin", "Edited plugin");
    assert!(matches!(
        import_edited_revision(&catalog, &root.path().join("drafts"), &descriptor, &edited,)
            .unwrap(),
        ImportOutcome::Imported(revision) if revision.plugin_version == descriptor.revision.plugin_version
    ));
    assert_eq!(list(&catalog).unwrap().len(), 2);

    let changed = same.replace("1.0.0", "1.1.0");
    assert!(matches!(
        import_edited_revision(&catalog, &root.path().join("drafts"), &descriptor, &changed,)
            .unwrap(),
        ImportOutcome::Imported(_)
    ));
    assert_eq!(list(&catalog).unwrap().len(), 3);
}

#[test]
fn assigned_games_match_exact_revision_with_case_insensitive_digest() {
    let descriptor = PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: "org.example.plugin".into(),
            plugin_version: Version::parse("1.2.3").unwrap(),
            package_md5: "a".repeat(32),
            package_sha256: "b".repeat(64),
            imported_at: 0,
            root: PathBuf::from(r"C:\catalog\revision"),
        },
        manifest: Manifest::parse(
            r#"
manifestVersion = 1
pluginId = "org.example.plugin"
pluginVersion = "1.2.3"
label = "Example"
description = "Example plugin"
"#,
        )
        .unwrap(),
    };
    let matching = Version::parse("1.2.3").unwrap();
    let other = Version::parse("1.2.4").unwrap();
    let upper = "B".repeat(64);
    let lower = "b".repeat(64);
    let assignments = [
        AssignedRevision {
            game_id: 7,
            plugin_id: "org.example.plugin",
            version: &matching,
            sha256: &upper,
        },
        AssignedRevision {
            game_id: 3,
            plugin_id: "org.example.plugin",
            version: &matching,
            sha256: &lower,
        },
        AssignedRevision {
            game_id: 5,
            plugin_id: "org.example.plugin",
            version: &other,
            sha256: &lower,
        },
        AssignedRevision {
            game_id: 9,
            plugin_id: "org.other.plugin",
            version: &matching,
            sha256: &lower,
        },
        AssignedRevision {
            game_id: 7,
            plugin_id: "org.example.plugin",
            version: &matching,
            sha256: &lower,
        },
    ];
    assert_eq!(assigned_games(&descriptor, assignments), vec![3, 7]);
    assert!(assigned_games(&descriptor, std::iter::empty()).is_empty());
}
