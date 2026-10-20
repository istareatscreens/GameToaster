use std::fs;

use semver::Version;

use super::verify::{reject_protected, reject_unsafe_syntax, verify_path, volume_root};
use super::*;
use crate::plugin::manifest::{ConfigFile, CopyType, Manifest, PluginFile, WatchType};
use crate::support::paths::{path_key, paths_equal};

fn manifest(config: Vec<ConfigFile>, plugin: Vec<PluginFile>) -> Manifest {
    Manifest {
        manifest_version: 1,
        plugin_id: "org.example.paths".into(),
        plugin_version: Version::parse("1.0.0").unwrap(),
        label: "Paths".into(),
        description: "Path test".into(),
        is_patcher: false,
        game_launcher: None,
        executable_watch: None,
        executable_watch_type: None,
        runtime: None,
        config_files: config,
        plugin_files: plugin,
        install_scripts: Vec::new(),
    }
}

fn policy(root: &Path) -> PathPolicy {
    PathPolicy {
        application_root: root.join("app"),
        catalog_root: root.join("catalog"),
        state_root: root.join("state"),
    }
}

fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = crate::plugin::fixtures::canonical_tempdir();
    let package = root.path().join("package");
    let game = root.path().join("game");
    fs::create_dir_all(&package).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("game.exe"), b"game").unwrap();
    (root, package, game)
}

#[test]
fn game_folder_and_find_replace_are_deterministic() {
    let (root, package, game) = setup();
    fs::write(package.join("config.ini"), b"new").unwrap();
    fs::create_dir_all(game.join("A")).unwrap();
    fs::create_dir_all(game.join("b")).unwrap();
    fs::write(game.join("A/CONFIG.INI"), b"a").unwrap();
    fs::write(game.join("b/config.ini"), b"b").unwrap();
    let manifest = manifest(
        vec![ConfigFile {
            config_file_location: "config.ini".into(),
            config_file_copy_type: CopyType::FindAndReplace,
            config_file_replace_path: None,
            config_add_to_config_menu: true,
        }],
        Vec::new(),
    );
    let plan = build_plan(
        &package,
        &manifest,
        &"a".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(plan.files.len(), 2);
    assert!(path_key(&plan.files[0].destination) < path_key(&plan.files[1].destination));
    assert!(plan.files.iter().all(|file| file.add_to_config_menu));
}

#[test]
fn zero_match_requires_and_records_explicit_choice() {
    let (root, package, game) = setup();
    fs::write(package.join("new.ini"), b"new").unwrap();
    let manifest = manifest(
        vec![ConfigFile {
            config_file_location: "new.ini".into(),
            config_file_copy_type: CopyType::FindAndReplace,
            config_file_replace_path: None,
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    assert!(build_plan(
        &package,
        &manifest,
        &"b".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path())
    )
    .unwrap_err()
    .contains("decision required"));
    let mut choices = ResolutionChoices::default();
    choices
        .zero_match
        .insert(0, ZeroMatchChoice::CopyToGameFolder);
    let plan = build_plan(
        &package,
        &manifest,
        &"b".repeat(64),
        1,
        &game.join("game.exe"),
        &choices,
        &policy(root.path()),
    )
    .unwrap();
    assert!(paths_equal(
        &plan.files[0].destination,
        &game.join("new.ini")
    ));
}

#[test]
fn find_replace_in_directory_is_scoped_and_zero_match_is_explicit() {
    let (root, package, game) = setup();
    fs::write(package.join("config.ini"), b"new").unwrap();
    fs::create_dir_all(game.join("scope/deep")).unwrap();
    fs::create_dir_all(game.join("outside")).unwrap();
    fs::write(game.join("scope/deep/CONFIG.INI"), b"scoped").unwrap();
    fs::write(game.join("outside/config.ini"), b"outside").unwrap();
    let scoped = manifest(
        vec![ConfigFile {
            config_file_location: "config.ini".into(),
            config_file_copy_type: CopyType::FindAndReplaceInDirectory,
            config_file_replace_path: Some("unused/../scope".into()),
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    let plan = build_plan(
        &package,
        &scoped,
        &"1".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(plan.files.len(), 1);
    assert!(paths_equal(
        &plan.files[0].destination,
        &game.join("scope/deep/CONFIG.INI")
    ));

    fs::write(package.join("missing.ini"), b"missing").unwrap();
    let missing = manifest(
        vec![ConfigFile {
            config_file_location: "missing.ini".into(),
            config_file_copy_type: CopyType::FindAndReplaceInDirectory,
            config_file_replace_path: Some("scope".into()),
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    let error = build_plan(
        &package,
        &missing,
        &"2".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap_err();
    assert!(error.contains("decision required"), "{error}");

    let mut no_action = ResolutionChoices::default();
    no_action.zero_match.insert(0, ZeroMatchChoice::NoAction);
    let plan = build_plan(
        &package,
        &missing,
        &"2".repeat(64),
        1,
        &game.join("game.exe"),
        &no_action,
        &policy(root.path()),
    )
    .unwrap();
    assert!(plan.files.is_empty());

    let mut fallback = ResolutionChoices::default();
    fallback
        .zero_match
        .insert(0, ZeroMatchChoice::CopyToGameFolder);
    let plan = build_plan(
        &package,
        &missing,
        &"2".repeat(64),
        1,
        &game.join("game.exe"),
        &fallback,
        &policy(root.path()),
    )
    .unwrap();
    assert!(paths_equal(
        &plan.files[0].destination,
        &game.join("missing.ini")
    ));
}

#[test]
fn absolute_mode_handles_exact_missing_and_consented_directory_forms() {
    let (root, package, game) = setup();
    fs::write(package.join("settings.ini"), b"new").unwrap();
    fs::write(game.join("exact.ini"), b"old").unwrap();

    for (replace_path, expected, existed) in [
        ("exact.ini", game.join("exact.ini"), true),
        ("new/leaf.ini", game.join("new/leaf.ini"), false),
    ] {
        let exact = manifest(
            vec![ConfigFile {
                config_file_location: "settings.ini".into(),
                config_file_copy_type: CopyType::Absolute,
                config_file_replace_path: Some(replace_path.into()),
                config_add_to_config_menu: false,
            }],
            Vec::new(),
        );
        let plan = build_plan(
            &package,
            &exact,
            &"3".repeat(64),
            1,
            &game.join("game.exe"),
            &ResolutionChoices::default(),
            &policy(root.path()),
        )
        .unwrap();
        assert!(paths_equal(&plan.files[0].destination, &expected));
        assert_eq!(plan.files[0].destination_existed, existed);
    }

    fs::create_dir_all(game.join("configs/deep")).unwrap();
    fs::write(game.join("configs/deep/settings.ini"), b"old").unwrap();
    let directory = manifest(
        vec![ConfigFile {
            config_file_location: "settings.ini".into(),
            config_file_copy_type: CopyType::Absolute,
            config_file_replace_path: Some("configs\\".into()),
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    let error = build_plan(
        &package,
        &directory,
        &"4".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap_err();
    assert!(
        error.contains("absolute destination is a directory"),
        "{error}"
    );

    let mut search = ResolutionChoices::default();
    search
        .directory_fallback
        .insert(0, DirectoryFallbackChoice::SearchDirectory);
    let plan = build_plan(
        &package,
        &directory,
        &"4".repeat(64),
        1,
        &game.join("game.exe"),
        &search,
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(plan.files.len(), 1);
    assert!(paths_equal(
        &plan.files[0].destination,
        &game.join("configs/deep/settings.ini")
    ));

    let mut no_action = ResolutionChoices::default();
    no_action
        .directory_fallback
        .insert(0, DirectoryFallbackChoice::NoAction);
    let plan = build_plan(
        &package,
        &directory,
        &"4".repeat(64),
        1,
        &game.join("game.exe"),
        &no_action,
        &policy(root.path()),
    )
    .unwrap();
    assert!(plan.files.is_empty());
}

#[test]
fn parent_relative_external_destination_needs_bound_grant() {
    let (root, package, game) = setup();
    let shared = root.path().join("shared");
    fs::create_dir_all(&shared).unwrap();
    fs::write(package.join("config.ini"), b"new").unwrap();
    let manifest = manifest(
        vec![ConfigFile {
            config_file_location: "config.ini".into(),
            config_file_copy_type: CopyType::Absolute,
            config_file_replace_path: Some("../shared/config.ini".into()),
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    let revision = "c".repeat(64);
    let preview = build_plan_preview(
        &package,
        &manifest,
        &revision,
        9,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(preview.required_external_grants.len(), 1);
    assert!(preview
        .clone()
        .approve(&[], None, None)
        .unwrap_err()
        .contains("not granted"));
    let grant_root = preview.required_external_grants[0].destination_root.clone();
    let (approved, grants) = preview.approve(&[grant_root], None, None).unwrap();
    assert_eq!(approved.files.len(), 1);
    assert_eq!(grants.len(), 1);
    let error = build_plan(
        &package,
        &manifest,
        &revision,
        9,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap_err();
    assert!(error.contains("grant"), "{error}");
    let verified = verify_path(&shared).unwrap();
    let choices = ResolutionChoices {
        external_grants: vec![ExternalGrant {
            plugin_id: manifest.plugin_id.clone(),
            game_id: 9,
            revision_sha256: revision.clone(),
            destination_root: verified.final_path,
            root_identity: verified.identity,
        }],
        ..ResolutionChoices::default()
    };
    assert!(build_plan(
        &package,
        &manifest,
        &revision,
        9,
        &game.join("game.exe"),
        &choices,
        &policy(root.path()),
    )
    .is_ok());
    let mut stale = choices;
    stale.external_grants[0].revision_sha256 = "d".repeat(64);
    assert!(build_plan(
        &package,
        &manifest,
        &revision,
        9,
        &game.join("game.exe"),
        &stale,
        &policy(root.path()),
    )
    .is_err());
}

#[test]
fn ambiguous_launcher_is_previewed_and_only_a_listed_choice_can_finalize() {
    let (root, package, game) = setup();
    fs::write(package.join("runner.exe"), b"runner").unwrap();
    for folder in ["one", "two"] {
        fs::create_dir_all(game.join(folder)).unwrap();
        fs::write(game.join(folder).join("runner.exe"), folder.as_bytes()).unwrap();
    }
    let mut manifest = manifest(
        Vec::new(),
        vec![PluginFile {
            plugin_file_location: "runner.exe".into(),
            plugin_file_copy_type: CopyType::FindAndReplace,
            plugin_file_replace_path: None,
        }],
    );
    manifest.game_launcher = Some("runner.exe".into());
    let preview = build_plan_preview(
        &package,
        &manifest,
        &"9".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(preview.ambiguities.len(), 1);
    assert_eq!(preview.ambiguities[0].kind, AmbiguityKind::Launcher);
    assert_eq!(preview.ambiguities[0].candidates.len(), 2);
    assert!(preview
        .clone()
        .approve(&[], None, None)
        .unwrap_err()
        .contains("explicit"));
    let selected = preview.ambiguities[0].candidates[1].clone();
    let (plan, _) = preview.approve(&[], Some(&selected), None).unwrap();
    assert!(plan
        .resolved_launcher
        .as_ref()
        .is_some_and(|path| paths_equal(path, &selected)));
}

#[test]
fn executable_watch_modes_preserve_selection_and_surface_ambiguity() {
    let (root, package, game) = setup();
    for folder in ["one", "two"] {
        fs::create_dir_all(game.join(folder)).unwrap();
        fs::write(game.join(folder).join("watch.exe"), folder.as_bytes()).unwrap();
    }
    let mut watched = manifest(Vec::new(), Vec::new());
    watched.executable_watch = Some("watch.exe".into());
    watched.executable_watch_type = Some(WatchType::FindInDirectory);
    let preview = build_plan_preview(
        &package,
        &watched,
        &"5".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert_eq!(preview.ambiguities.len(), 1);
    assert_eq!(preview.ambiguities[0].kind, AmbiguityKind::WatchTarget);
    let selected = preview.ambiguities[0].candidates[0].clone();
    let (plan, _) = preview.approve(&[], None, Some(&selected)).unwrap();
    assert!(plan
        .resolved_watch_target
        .as_ref()
        .is_some_and(|path| paths_equal(path, &selected)));

    watched.executable_watch = None;
    watched.executable_watch_type = Some(WatchType::GameDirectory);
    let plan = build_plan(
        &package,
        &watched,
        &"6".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    assert!(plan
        .resolved_watch_target
        .as_ref()
        .is_some_and(|path| paths_equal(path, &plan.selected_target)));
}

#[test]
fn protected_roots_device_namespaces_and_ads_are_unconditionally_denied() {
    let (root, _package, game) = setup();
    let policy = policy(root.path());
    for protected in [
        policy.application_root.join("payload.bin"),
        policy.catalog_root.join("payload.bin"),
        policy.state_root.join("payload.bin"),
    ] {
        let error = reject_protected(&protected, &policy).unwrap_err();
        assert!(error.contains("protected root"), "{error}");
    }
    let drive_root = volume_root(root.path()).unwrap();
    assert!(reject_protected(&drive_root, &policy)
        .unwrap_err()
        .contains("drive-root"));
    assert!(reject_unsafe_syntax(Path::new(r"\\.\PhysicalDrive0"))
        .unwrap_err()
        .contains("namespaces"));
    assert!(reject_unsafe_syntax(&game.join("config.ini:stream"))
        .unwrap_err()
        .contains("alternate data streams"));
}

#[test]
fn case_colliding_destinations_are_refused() {
    let (root, package, game) = setup();
    fs::write(package.join("A.ini"), b"a").unwrap();
    fs::write(package.join("b.ini"), b"b").unwrap();
    let manifest = manifest(
        vec![
            ConfigFile {
                config_file_location: "A.ini".into(),
                config_file_copy_type: CopyType::Absolute,
                config_file_replace_path: Some("same.ini".into()),
                config_add_to_config_menu: false,
            },
            ConfigFile {
                config_file_location: "b.ini".into(),
                config_file_copy_type: CopyType::Absolute,
                config_file_replace_path: Some("SAME.INI".into()),
                config_add_to_config_menu: false,
            },
        ],
        Vec::new(),
    );
    assert!(build_plan(
        &package,
        &manifest,
        &"e".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap_err()
    .contains("same destination"));
}

#[test]
fn revalidation_refuses_destination_swap() {
    let (root, package, game) = setup();
    fs::write(package.join("config.ini"), b"new").unwrap();
    fs::write(game.join("config.ini"), b"old").unwrap();
    let manifest = manifest(
        Vec::new(),
        vec![PluginFile {
            plugin_file_location: "config.ini".into(),
            plugin_file_copy_type: CopyType::GameFolder,
            plugin_file_replace_path: None,
        }],
    );
    let plan = build_plan(
        &package,
        &manifest,
        &"f".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();
    fs::remove_file(game.join("config.ini")).unwrap();
    fs::write(game.join("config.ini"), b"swapped").unwrap();
    assert!(revalidate_plan(&plan, &policy(root.path()))
        .unwrap_err()
        .contains("changed after preview"));
}

#[cfg(windows)]
#[test]
fn revalidation_refuses_a_parent_swapped_to_a_junction() {
    let (root, package, game) = setup();
    fs::write(package.join("config.ini"), b"new").unwrap();
    fs::create_dir_all(game.join("sub")).unwrap();
    let manifest = manifest(
        vec![ConfigFile {
            config_file_location: "config.ini".into(),
            config_file_copy_type: CopyType::Absolute,
            config_file_replace_path: Some("sub/config.ini".into()),
            config_add_to_config_menu: false,
        }],
        Vec::new(),
    );
    let plan = build_plan(
        &package,
        &manifest,
        &"7".repeat(64),
        1,
        &game.join("game.exe"),
        &ResolutionChoices::default(),
        &policy(root.path()),
    )
    .unwrap();

    fs::remove_dir(game.join("sub")).unwrap();
    let redirect = root.path().join("redirect");
    fs::create_dir_all(&redirect).unwrap();
    let output = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(game.join("sub"))
        .arg(&redirect)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "mklink failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error = revalidate_plan(&plan, &policy(root.path())).unwrap_err();
    assert!(error.contains("reparse traversal"), "{error}");
    fs::remove_dir(game.join("sub")).unwrap();
}

#[cfg(windows)]
#[test]
fn canonical_verbatim_path_starts_verification_at_complete_volume_root() {
    let (_root, _package, game) = setup();
    let canonical = fs::canonicalize(&game).unwrap();
    assert!(canonical.to_string_lossy().starts_with(r"\\?\"));

    let verified = verify_path(&canonical).unwrap();

    assert!(verified.identity.is_some());
    assert!(verified.final_path.is_dir());
    assert_eq!(
        verified.final_path.file_name(),
        canonical.file_name(),
        "handle normalization must retain the selected directory"
    );
}
