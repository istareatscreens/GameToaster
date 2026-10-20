use super::*;

fn config(exe: &str) -> LauncherConfig {
    LauncherConfig {
        settings: LauncherSettings::default(),
        default_game: None,
        next_game_id: 2,
        games: vec![GameEntry {
            id: GameId(1),
            label: "Game".into(),
            controller_image: None,
            selected_target: PathBuf::from(exe),
            plugin: None,
            hotkey: None,
            watch_exe: None,
            close_mode: CloseMode::KillExe,
            close_script: None,
            close_hotkey: None,
            config_targets: Vec::new(),
        }],
    }
}

#[test]
fn launchable_targets_are_executables_and_scripts() {
    assert!(is_launchable_extension(Path::new(r"C:\g\game.EXE")));
    assert!(is_launchable_extension(Path::new(r"C:\g\run.bat")));
    assert!(is_launchable_extension(Path::new(r"C:\g\run.cmd")));
    assert!(!is_launchable_extension(Path::new(r"C:\g\readme.txt")));
}

#[test]
fn plugin_watch_keeps_the_exact_selected_executable_path() {
    let mut game = config(r"C:\Games\Mai\selected.exe").games.remove(0);
    game.plugin = Some(PluginAssignment {
        plugin_id: "org.example.plugin".into(),
        revision: PluginRevision {
            version: semver::Version::new(1, 0, 0),
            sha256: "a".repeat(64),
        },
        update_policy: PluginUpdatePolicy::Pinned,
        allow_prerelease: false,
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        assignment_id: "b".repeat(32),
        resolved_launcher: None,
        resolved_watch_target: Some(game.selected_target.clone()),
        grants: Vec::new(),
        ledger_reference: PathBuf::from(r"C:\state\ledger.toml"),
    });

    assert_eq!(game.watch_target().as_deref(), Some("selected.exe"));
    assert_eq!(
        game.watch_target_path().as_deref(),
        Some(Path::new(r"C:\Games\Mai\selected.exe"))
    );
}

#[test]
fn close_plan_decision_tree() {
    let base = |exe: &str| GameEntry {
        id: GameId(1),
        label: "G".into(),
        controller_image: None,
        selected_target: PathBuf::from(exe),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    };

    let g = GameEntry {
        close_mode: CloseMode::Script,
        close_script: Some(PathBuf::from(r"C:\g\close.bat")),
        ..base(r"C:\g\game.exe")
    };
    assert_eq!(
        g.close_plan(),
        ClosePlan::RunScript(PathBuf::from(r"C:\g\close.bat"))
    );

    let g = GameEntry {
        close_mode: CloseMode::Script,
        ..base(r"C:\g\game.exe")
    };
    assert_eq!(g.close_plan(), ClosePlan::Nothing);

    let g = GameEntry {
        watch_exe: Some("watch.exe".into()),
        ..base(r"C:\g\game.exe")
    };
    assert_eq!(g.close_plan(), ClosePlan::Kill("watch.exe".into()));

    assert_eq!(
        base(r"C:\g\game.exe").close_plan(),
        ClosePlan::Kill("game.exe".into())
    );

    assert_eq!(base(r"C:\g\run.bat").close_plan(), ClosePlan::Nothing);

    let g = GameEntry {
        watch_exe: Some("   ".into()),
        ..base(r"C:\g\game.exe")
    };
    assert_eq!(g.close_plan(), ClosePlan::Kill("game.exe".into()));
}

#[test]
fn scripts_and_executables_validate_without_game_specific_kinds() {
    assert!(config(r"C:\g\run.bat").validate().is_ok());
    assert!(config(r"C:\g\run.cmd").validate().is_ok());
    assert!(config(r"C:\g\game.exe").validate().is_ok());
    let invalid = config(r"C:\g\notes.txt").validate().unwrap_err();
    assert!(invalid.contains(".exe"));
}

#[test]
fn bare_watch_name_is_matched_by_name_not_as_a_game_folder_path() {
    let mut game = config(r"C:\Games\Example\s.bat").games.remove(0);
    game.watch_exe = Some("client.exe".into());
    assert_eq!(game.watch_target().as_deref(), Some("client.exe"));
    assert_eq!(game.watch_target_path(), None);

    game.watch_exe = Some(r"Package\client.exe".into());
    assert_eq!(
        game.watch_target_path().as_deref(),
        Some(Path::new(r"C:\Games\Example\Package\client.exe"))
    );

    game.watch_exe = Some(r"D:\Elsewhere\client.exe".into());
    assert_eq!(
        game.watch_target_path().as_deref(),
        Some(Path::new(r"D:\Elsewhere\client.exe"))
    );

    game.watch_exe = Some("   ".into());
    assert_eq!(game.watch_target(), None);
    assert_eq!(game.watch_target_path(), None);
}

#[test]
fn the_update_policy_follows_the_tracking_choice_and_its_text() {
    assert_eq!(
        PluginUpdatePolicy::for_tracking(true),
        PluginUpdatePolicy::TrackLatest
    );
    assert_eq!(
        PluginUpdatePolicy::for_tracking(false),
        PluginUpdatePolicy::Pinned
    );
    for policy in [PluginUpdatePolicy::TrackLatest, PluginUpdatePolicy::Pinned] {
        assert_eq!(
            PluginUpdatePolicy::for_tracking(policy.tracks_latest()),
            policy
        );
        assert_eq!(
            policy.to_string().parse::<PluginUpdatePolicy>().unwrap(),
            policy
        );
    }
    assert!("latest".parse::<PluginUpdatePolicy>().is_err());
}

#[test]
fn the_automatic_watch_name_yields_to_a_plugins_target_and_an_operators_name_does_not() {
    use crate::launcher_gui::model::automatic_watch_name;
    assert_eq!(
        automatic_watch_name(Path::new(r"C:\Games\Arcade\Game.EXE")).as_deref(),
        Some("Game.EXE")
    );
    assert_eq!(automatic_watch_name(Path::new(r"C:\Games\start.bat")), None);

    let plugin_target = PathBuf::from(r"C:\Games\Arcade\client.exe");
    let mut game = config(r"C:\Games\Arcade\game.exe").games.remove(0);
    let assignment = |target: &Path| PluginAssignment {
        plugin_id: "org.example.plugin".into(),
        revision: PluginRevision {
            version: semver::Version::parse("1.0.0").unwrap(),
            sha256: "f".repeat(64),
        },
        update_policy: PluginUpdatePolicy::TrackLatest,
        allow_prerelease: false,
        copy_zero_matches_to_game_folder: false,
        search_directory_destinations: false,
        assignment_id: "0123456789abcdef0123456789abcdef".into(),
        resolved_launcher: None,
        resolved_watch_target: Some(target.to_path_buf()),
        grants: Vec::new(),
        ledger_reference: PathBuf::from(r"C:\state\ledger.toml"),
    };

    game.watch_exe = Some(" GAME.exe ".into());
    assert_eq!(game.watch_target().as_deref(), Some("GAME.exe"));
    assert_eq!(game.watch_target_path(), None);
    assert_eq!(game.stored_automatic_watch().as_deref(), Some("GAME.exe"));

    game.plugin = Some(assignment(&plugin_target));
    assert_eq!(game.watch_target().as_deref(), Some("client.exe"));
    assert_eq!(
        game.watch_target_path().as_deref(),
        Some(plugin_target.as_path())
    );
    assert_eq!(game.stored_automatic_watch().as_deref(), Some("GAME.exe"));
    game.plugin = None;
    assert_eq!(game.watch_target().as_deref(), Some("GAME.exe"));

    game.plugin = Some(assignment(&plugin_target));
    game.watch_exe = Some("launcher.exe".into());
    assert_eq!(game.watch_target().as_deref(), Some("launcher.exe"));
    assert_eq!(game.stored_automatic_watch(), None);
    game.watch_exe = Some(r".\game.exe".into());
    assert_eq!(game.watch_target().as_deref(), Some(r".\game.exe"));
    assert_eq!(
        game.watch_target_path().as_deref(),
        Some(Path::new(r"C:\Games\Arcade\.\game.exe"))
    );

    game.watch_exe = None;
    assert_eq!(game.watch_target().as_deref(), Some("client.exe"));
    game.plugin = None;
    assert_eq!(game.watch_target(), None);
    assert_eq!(game.watch_target_path(), None);
}
