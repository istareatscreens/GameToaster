use std::collections::BTreeMap;

use semver::Version;

use super::*;
use crate::plugin::catalog::ImportOutcome;
use crate::plugin::manifest::{ConfigFile, CopyType, Manifest, PluginFile, Runtime};
use crate::plugin::paths::{PathPolicy, ResolutionChoices};

struct Fixture {
    _root: tempfile::TempDir,
    catalog: Catalog,
    service: LifecycleService,
    source_root: PathBuf,
    game_one: PathBuf,
    game_two: PathBuf,
}

fn write_package(root: &Path, version: &str, bytes: &[u8]) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("payload.bin"), bytes).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: "org.example.lifecycle".into(),
        plugin_version: Version::parse(version).unwrap(),
        label: "Lifecycle".into(),
        description: "Lifecycle test plugin".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: Vec::new(),
        plugin_files: vec![PluginFile {
            plugin_file_location: "payload.bin".into(),
            plugin_file_copy_type: CopyType::GameFolder,
            plugin_file_replace_path: None,
        }],
        install_scripts: Vec::new(),
    };
    fs::write(
        root.join("manifest.toml"),
        manifest.canonical_toml().unwrap(),
    )
    .unwrap();
}

fn import(catalog: &Catalog, root: &Path, version: &str, bytes: &[u8]) -> CatalogRevision {
    write_package(root, version, bytes);
    match catalog
        .import_manifest(&root.join("manifest.toml"))
        .unwrap()
    {
        ImportOutcome::Imported(revision) | ImportOutcome::AlreadyPresent(revision) => revision,
    }
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let catalog_root = root.path().join("catalog");
    let state_root = root.path().join("state");
    let app_root = root.path().join("app");
    let game_one = root.path().join("game-one");
    let game_two = root.path().join("game-two");
    for path in [&app_root, &game_one, &game_two] {
        fs::create_dir_all(path).unwrap();
    }
    fs::write(game_one.join("game.exe"), b"game-one").unwrap();
    fs::write(game_two.join("game.exe"), b"game-two").unwrap();
    let catalog = Catalog::open(&catalog_root).unwrap();
    let source_root = root.path().join("source-v1");
    import(&catalog, &source_root, "1.0.0", b"one");
    let deployment = DeploymentEngine::new(
        &state_root,
        PathPolicy {
            application_root: app_root,
            catalog_root,
            state_root: state_root.clone(),
        },
    )
    .unwrap();
    let service = LifecycleService::new(catalog.clone(), deployment).unwrap();
    Fixture {
        _root: root,
        catalog,
        service,
        source_root,
        game_one,
        game_two,
    }
}

fn request(
    game_id: u64,
    game: &Path,
    assignment: char,
    policy: RevisionPolicy,
) -> ReconcileRequest {
    ReconcileRequest {
        assignment_id: assignment.to_string().repeat(32),
        plugin_id: "org.example.lifecycle".into(),
        game_id,
        selected_target: game.join("game.exe"),
        policy,
        choices: ResolutionChoices::default(),
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        modification_decisions: BTreeMap::new(),
    }
}

#[test]
fn activity_guard_pauses_all_plugin_work_until_process_exit() {
    let fixture = fixture();
    let work = fixture.service.activity().try_plugin_operation().unwrap();
    assert!(!fixture.service.watcher_running());
    assert!(fixture.service.activity().try_plugin_operation().is_err());
    let game = work.reserve_game().unwrap();
    assert_eq!(game.active_game_count(), 1);
    assert!(fixture.service.activity().try_plugin_operation().is_err());
    drop(game);
    assert_eq!(fixture.service.activity().active_game_count(), 0);
    assert!(fixture.service.watcher_running());
}

#[test]
fn full_rescan_handles_missed_events_pins_prereleases_and_two_games() {
    let fixture = fixture();
    let first = fixture
        .service
        .reconcile_for_launch(request(
            1,
            &fixture.game_one,
            'a',
            RevisionPolicy::TrackLatest {
                allow_prerelease: false,
            },
        ))
        .unwrap();
    let v1_sha = first.reconciliation.revision.package_sha256.clone();
    assert!(first.reconciliation.changed);
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"one"
    );

    let source_v2 = fixture.source_root.with_file_name("source-v2");
    import(&fixture.catalog, &source_v2, "2.0.0", b"two");
    assert!(fixture.service.activity().try_plugin_operation().is_err());
    first.finish();

    let pinned = fixture
        .service
        .reconcile_for_launch(request(
            2,
            &fixture.game_two,
            'b',
            RevisionPolicy::Pinned {
                version: Version::parse("1.0.0").unwrap(),
                sha256: v1_sha,
            },
        ))
        .unwrap();
    assert_eq!(
        pinned.reconciliation.revision.plugin_version,
        Version::parse("1.0.0").unwrap()
    );
    assert_eq!(
        fs::read(fixture.game_two.join("payload.bin")).unwrap(),
        b"one"
    );
    let receipt_two = pinned
        .reconciliation
        .ledger
        .game_folder
        .join(".GameToaster")
        .join("b".repeat(32))
        .join("receipt.toml");
    assert!(receipt_two.is_file());
    pinned.finish();

    let updated = fixture
        .service
        .reconcile_for_launch(request(
            1,
            &fixture.game_one,
            'a',
            RevisionPolicy::TrackLatest {
                allow_prerelease: false,
            },
        ))
        .unwrap();
    assert_eq!(
        updated.reconciliation.revision.plugin_version,
        Version::parse("2.0.0").unwrap()
    );
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"two"
    );
    assert_ne!(
        updated.reconciliation.ledger_reference,
        receipt_two.parent().unwrap().join("ledger.toml")
    );
    updated.finish();

    let source_v3 = fixture.source_root.with_file_name("source-v3-beta");
    import(&fixture.catalog, &source_v3, "3.0.0-beta.1", b"beta");
    let stable = fixture
        .service
        .reconcile_for_launch(request(
            1,
            &fixture.game_one,
            'a',
            RevisionPolicy::TrackLatest {
                allow_prerelease: false,
            },
        ))
        .unwrap();
    assert_eq!(
        stable.reconciliation.revision.plugin_version,
        Version::parse("2.0.0").unwrap()
    );
    stable.finish();
    let prerelease = fixture
        .service
        .reconcile_for_launch(request(
            1,
            &fixture.game_one,
            'a',
            RevisionPolicy::TrackLatest {
                allow_prerelease: true,
            },
        ))
        .unwrap();
    assert_eq!(
        prerelease.reconciliation.revision.plugin_version,
        Version::parse("3.0.0-beta.1").unwrap()
    );
    prerelease.finish();
}

const CONFIG_PLUGIN_ID: &str = "org.example.configowner";

fn write_config_package(root: &Path, version: &str, payload: &[u8], template: &[u8]) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("payload.bin"), payload).unwrap();
    fs::write(root.join("config.template.ini"), template).unwrap();
    let manifest = Manifest {
        manifest_version: 1,
        plugin_id: CONFIG_PLUGIN_ID.into(),
        plugin_version: Version::parse(version).unwrap(),
        label: "Config owner".into(),
        description: "Configuration ownership test plugin".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: vec![ConfigFile {
            config_file_location: "config.template.ini".into(),
            config_file_copy_type: CopyType::Absolute,
            config_file_replace_path: Some("./config.ini".into()),
            config_add_to_config_menu: true,
        }],
        plugin_files: vec![PluginFile {
            plugin_file_location: "payload.bin".into(),
            plugin_file_copy_type: CopyType::GameFolder,
            plugin_file_replace_path: None,
        }],
        install_scripts: Vec::new(),
    };
    fs::write(
        root.join("manifest.toml"),
        manifest.canonical_toml().unwrap(),
    )
    .unwrap();
}

fn import_config_package(catalog: &Catalog, root: &Path, version: &str, payload: &[u8]) {
    write_config_package(
        root,
        version,
        payload,
        b"template-bytes
",
    );
    match catalog
        .import_manifest(&root.join("manifest.toml"))
        .unwrap()
    {
        ImportOutcome::Imported(revision) | ImportOutcome::AlreadyPresent(revision) => revision,
    };
}

fn config_request(game_id: u64, game: &Path, assignment: char) -> ReconcileRequest {
    ReconcileRequest {
        assignment_id: assignment.to_string().repeat(32),
        plugin_id: CONFIG_PLUGIN_ID.into(),
        game_id,
        selected_target: game.join("game.exe"),
        policy: RevisionPolicy::TrackLatest {
            allow_prerelease: false,
        },
        choices: ResolutionChoices::default(),
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        modification_decisions: BTreeMap::new(),
    }
}

#[test]
fn edited_configuration_never_refuses_a_launch_and_is_never_rewritten() {
    let fixture = fixture();
    let source = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source, "1.0.0", b"one");
    let config = fixture.game_one.join("config.ini");

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();
    assert_eq!(
        fs::read(&config).unwrap(),
        b"template-bytes
"
    );

    fs::write(
        &config,
        b"operator-tuned-bytes
",
    )
    .unwrap();

    for attempt in 0..3 {
        let prepared = fixture
            .service
            .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
            .unwrap_or_else(|error| {
                panic!("launch attempt {attempt} was refused: {error}");
            });
        prepared.finish();
        assert_eq!(
            fs::read(&config).unwrap(),
            b"operator-tuned-bytes
",
            "launch attempt {attempt} rewrote the operator configuration"
        );
    }
}

#[test]
fn plugin_update_keeps_the_operator_configuration_and_still_updates_binaries() {
    let fixture = fixture();
    let source_one = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source_one, "1.0.0", b"one");
    let config = fixture.game_one.join("config.ini");

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();
    fs::write(
        &config,
        b"operator-tuned-bytes
",
    )
    .unwrap();

    let source_two = fixture.source_root.with_file_name("source-config-v2");
    import_config_package(&fixture.catalog, &source_two, "2.0.0", b"two");
    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .expect("an update must not be refused by an edited configuration")
        .finish();

    assert_eq!(
        fs::read(&config).unwrap(),
        b"operator-tuned-bytes
"
    );
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"two"
    );
}

#[test]
fn a_deleted_configuration_is_reseeded_on_the_next_launch() {
    let fixture = fixture();
    let source = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source, "1.0.0", b"one");
    let config = fixture.game_one.join("config.ini");

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();
    fs::remove_file(&config).unwrap();

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();
    assert_eq!(
        fs::read(&config).unwrap(),
        b"template-bytes
"
    );
}

#[test]
fn a_ledger_recording_the_template_hash_loads_current_and_writes_nothing() {
    let fixture = fixture();
    let source = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source, "1.0.0", b"one");
    let config = fixture.game_one.join("config.ini");

    let installed = fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap();
    let ledger_path = installed.reconciliation.ledger_reference.clone();
    installed.finish();

    let template_hash = sha256_file(&config).unwrap();
    let recorded = fs::read_to_string(&ledger_path).unwrap();
    assert!(
        recorded.contains(&template_hash),
        "the ledger must record the deployed configuration hash"
    );

    fs::write(&config, b"operator-tuned-bytes\n").unwrap();
    let before = fs::metadata(&config).unwrap().modified().unwrap();

    let prepared = fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .expect("a recorded hash that no longer matches must not refuse the launch");
    assert!(
        !prepared.reconciliation.changed,
        "an operator-owned configuration must not make the assignment stale"
    );
    prepared.finish();

    assert_eq!(fs::read(&config).unwrap(), b"operator-tuned-bytes\n");
    assert_eq!(fs::metadata(&config).unwrap().modified().unwrap(), before);
}

#[test]
fn no_planner_can_demand_a_decision_for_a_configuration_file() {
    let fixture = fixture();
    let source = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source, "1.0.0", b"one");
    let config = fixture.game_one.join("config.ini");

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();
    fs::write(&config, b"operator-tuned-bytes\n").unwrap();

    match fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
    {
        Ok(prepared) => prepared.finish(),
        Err(error) => panic!("launch planner demanded a decision: {error}"),
    }

    let source_two = fixture.source_root.with_file_name("source-config-v2");
    import_config_package(&fixture.catalog, &source_two, "2.0.0", b"two");
    match fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
    {
        Ok(prepared) => prepared.finish(),
        Err(error) => panic!("update planner demanded a decision: {error}"),
    }
    assert_eq!(fs::read(&config).unwrap(), b"operator-tuned-bytes\n");
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"two",
        "the plugin binary must still be updated"
    );
}

#[test]
fn a_modified_plugin_binary_still_refuses_the_launch() {
    let fixture = fixture();
    let source = fixture.source_root.with_file_name("source-config-v1");
    import_config_package(&fixture.catalog, &source, "1.0.0", b"one");

    fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
        .unwrap()
        .finish();

    fs::write(fixture.game_one.join("payload.bin"), b"operator-swapped").unwrap();
    match fixture
        .service
        .reconcile_for_launch(config_request(1, &fixture.game_one, 'c'))
    {
        Ok(_) => panic!("a replaced plugin binary must still stop the launch"),
        Err(error) => assert!(
            error.contains("decision required"),
            "unexpected refusal reason: {error}"
        ),
    }
}

#[test]
fn the_runtime_executable_is_matched_by_its_package_source_in_any_case() {
    let runtime = Runtime {
        executable: "bin/Plugin.exe".into(),
        operations: Vec::new(),
    };
    let files = [
        ("bin/plugin.EXE", Path::new(r"D:\game\plugin.exe")),
        ("bin/other.exe", Path::new(r"D:\game\other.exe")),
        ("", Path::new(r"D:\game\unnamed.exe")),
    ];
    assert_eq!(
        runtime_executable_candidates(&runtime, files).unwrap(),
        [PathBuf::from(r"D:\game\plugin.exe")]
    );
    let unnamed = Runtime {
        executable: "  ".into(),
        ..runtime
    };
    assert!(runtime_executable_candidates(&unnamed, files).is_err());
}

#[test]
fn a_tracking_game_moves_to_a_rebuilt_package_of_the_same_version_and_a_pinned_one_stays() {
    let fixture = fixture();
    let track = || {
        request(
            1,
            &fixture.game_one,
            'a',
            RevisionPolicy::TrackLatest {
                allow_prerelease: false,
            },
        )
    };
    let first = fixture.service.reconcile_for_launch(track()).unwrap();
    let first_sha = first.reconciliation.revision.package_sha256.clone();
    first.finish();
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"one"
    );

    std::thread::sleep(std::time::Duration::from_millis(2));
    let rebuilt = import(
        &fixture.catalog,
        &fixture.source_root.with_file_name("source-v1-rebuilt"),
        "1.0.0",
        b"one-rebuilt",
    );
    assert_ne!(rebuilt.package_sha256, first_sha);

    let moved = fixture.service.reconcile_for_launch(track()).unwrap();
    assert_eq!(
        moved.reconciliation.revision.package_sha256,
        rebuilt.package_sha256
    );
    assert!(moved.reconciliation.changed);
    moved.finish();
    assert_eq!(
        fs::read(fixture.game_one.join("payload.bin")).unwrap(),
        b"one-rebuilt",
        "the tracking game runs the rebuilt files at its next launch"
    );

    let pinned = fixture
        .service
        .reconcile_for_launch(request(
            2,
            &fixture.game_two,
            'b',
            RevisionPolicy::Pinned {
                version: Version::parse("1.0.0").unwrap(),
                sha256: first_sha.clone(),
            },
        ))
        .unwrap();
    assert_eq!(pinned.reconciliation.revision.package_sha256, first_sha);
    pinned.finish();
    assert_eq!(
        fs::read(fixture.game_two.join("payload.bin")).unwrap(),
        b"one",
        "a pinned game keeps its digest"
    );
}
