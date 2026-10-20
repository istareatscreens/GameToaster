use super::*;
use crate::plugin::catalog::library::PluginDescriptor;
use crate::plugin::catalog::CatalogRevision;
use crate::plugin::manifest::Manifest;
use semver::Version;

fn descriptor() -> PluginDescriptor {
    let manifest = Manifest::parse(
        r#"
manifestVersion = 1
pluginId = "org.example.ui"
pluginVersion = "1.2.3"
label = "UI fixture"
description = "Selection fixture"
isPatcher = false
"#,
    )
    .unwrap();
    PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            package_md5: "a".repeat(32),
            package_sha256: "b".repeat(64),
            imported_at: 0,
            root: PathBuf::from(r"C:\catalog\ui"),
        },
        manifest,
    }
}

#[test]
fn add_game_defaults_to_no_plugin_and_preserves_explicit_revision_policy() {
    let mut draft = AddDraft::new(
        PathBuf::from(r"C:\games\game.exe"),
        "Game".into(),
        vec![descriptor()],
    );
    assert!(draft.plugin_choice.selection().is_none());

    draft.plugin_choice.selected = Some(0);
    draft.plugin_choice.track_latest = false;
    draft.plugin_choice.allow_prerelease = true;
    draft.plugin_choice.copy_zero_matches_to_game_folder = true;
    draft.plugin_choice.search_directory_destinations = true;
    let selection = draft.plugin_choice.selection().unwrap();
    assert_eq!(selection.plugin_id, "org.example.ui");
    assert_eq!(selection.version, Version::parse("1.2.3").unwrap());
    assert_eq!(selection.sha256, "b".repeat(64));
    assert!(!selection.track_latest);
    assert!(selection.allow_prerelease);
    assert!(selection.copy_zero_matches_to_game_folder);
    assert!(selection.search_directory_destinations);
}

#[test]
fn game_settings_selection_uses_only_the_chosen_revision_and_policy() {
    let mut draft = GameSettingsDraft {
        id: GameId(7),
        label: "Game".into(),
        watch_exe: String::new(),
        close_mode: CloseMode::KillExe,
        close_script: None,
        config_targets: Vec::new(),
        store_picker: None,
        store_filter: String::new(),
        plugin: None,
        plugin_choice: PluginChoice::new(vec![descriptor()]),
        plugin_inspection: None,
        confirm_config_reset: false,
        resolved_watch_name: None,
        automatic_watch_name: None,
        stored_automatic_watch: None,
    };
    assert!(draft.plugin_choice.selection().is_none());

    draft.plugin_choice.selected = Some(0);
    draft.plugin_choice.track_latest = false;
    let selection = draft.plugin_choice.selection().unwrap();
    assert_eq!(selection.plugin_id, "org.example.ui");
    assert_eq!(selection.version, Version::parse("1.2.3").unwrap());
    assert_eq!(selection.sha256, "b".repeat(64));
    assert!(!selection.track_latest);
}

fn watch_draft(watch_exe: &str, resolved: Option<&str>) -> GameSettingsDraft {
    GameSettingsDraft {
        id: GameId(7),
        label: "Game".into(),
        watch_exe: watch_exe.into(),
        close_mode: CloseMode::KillExe,
        close_script: None,
        config_targets: Vec::new(),
        store_picker: None,
        store_filter: String::new(),
        plugin: None,
        plugin_choice: PluginChoice::new(Vec::new()),
        plugin_inspection: None,
        confirm_config_reset: false,
        resolved_watch_name: resolved.map(str::to_owned),
        automatic_watch_name: None,
        stored_automatic_watch: None,
    }
}

#[test]
fn an_untouched_prefilled_watch_field_is_not_persisted_as_an_override() {
    let draft = watch_draft("game.exe", Some("game.exe"));
    assert_eq!(draft.watch_override(), None);

    let draft = watch_draft("  GAME.EXE ", Some("game.exe"));
    assert_eq!(draft.watch_override(), None);
}

#[test]
fn a_typed_watch_name_still_wins_and_clearing_returns_to_the_resolved_target() {
    let draft = watch_draft("other_game.exe", Some("game.exe"));
    assert_eq!(draft.watch_override().as_deref(), Some("other_game.exe"));

    let draft = watch_draft("", Some("game.exe"));
    assert_eq!(draft.watch_override(), None);
}

#[test]
fn without_a_plugin_the_watch_field_is_a_plain_optional_override() {
    assert_eq!(watch_draft("", None).watch_override(), None);
    assert_eq!(
        watch_draft("game.exe", None).watch_override().as_deref(),
        Some("game.exe")
    );
}

fn descriptor_watching(executable_watch: &str) -> PluginDescriptor {
    let manifest = Manifest::parse(&format!(
        r#"
manifestVersion = 1
pluginId = "org.example.watch"
pluginVersion = "1.0.0"
label = "Watch fixture"
description = "Watch fixture"
executableWatch = "{executable_watch}"
"#
    ))
    .unwrap();
    PluginDescriptor {
        revision: CatalogRevision {
            plugin_id: manifest.plugin_id.clone(),
            plugin_version: manifest.plugin_version.clone(),
            package_md5: "c".repeat(32),
            package_sha256: "d".repeat(64),
            imported_at: 0,
            root: PathBuf::from(r"C:\catalog\watch"),
        },
        manifest,
    }
}

fn add_draft(path: &str, plugin: Option<PluginDescriptor>, watch_exe: &str) -> AddDraft {
    let selected = plugin.as_ref().map(|_| 0);
    let mut draft = AddDraft::new(
        PathBuf::from(path),
        "  Game  ".into(),
        plugin.into_iter().collect(),
    );
    draft.plugin_choice.selected = selected;
    draft.watch_exe = watch_exe.into();
    draft
}

#[test]
fn an_executable_fills_the_watch_field_with_its_own_name_and_a_batch_file_does_not() {
    let exe = AddDraft::new(
        PathBuf::from(r"C:\games\Game.EXE"),
        "Game".into(),
        Vec::new(),
    );
    assert_eq!(exe.watch_exe, "Game.EXE");
    assert_eq!(exe.watch_override().as_deref(), Some("Game.EXE"));
    assert_eq!(
        exe.game_entry(GameId(1)).watch_exe.as_deref(),
        Some("Game.EXE")
    );
    for script in [r"C:\games\start.bat", r"C:\games\start.cmd"] {
        let draft = AddDraft::new(PathBuf::from(script), "Game".into(), Vec::new());
        assert_eq!(draft.watch_exe, "", "{script}");
        assert_eq!(draft.game_entry(GameId(1)).watch_exe, None, "{script}");
    }
}

#[test]
fn a_plugins_resolved_target_replaces_an_untouched_automatic_name_and_is_not_stored() {
    let mut draft = AddDraft::new(
        PathBuf::from(r"C:\games\game.exe"),
        "Game".into(),
        vec![descriptor_watching("bin/client.exe")],
    );
    draft.sync_watch_default();
    assert_eq!(
        draft.watch_exe, "game.exe",
        "no plugin: the executable's own name"
    );
    draft.plugin_choice.selected = Some(0);
    draft.sync_watch_default();
    assert_eq!(draft.watch_exe, "client.exe");
    assert_eq!(
        draft.watch_override(),
        None,
        "the plugin's target stays derived"
    );
    draft.plugin_choice.selected = None;
    draft.sync_watch_default();
    assert_eq!(draft.watch_exe, "game.exe");
    assert_eq!(draft.watch_override().as_deref(), Some("game.exe"));
}

#[test]
fn an_edited_watch_name_survives_a_plugin_change() {
    let plugins = || vec![descriptor_watching("bin/client.exe")];
    let mut typed = AddDraft::new(
        PathBuf::from(r"C:\games\game.exe"),
        "Game".into(),
        plugins(),
    );
    typed.watch_exe = "launcher.exe".into();
    typed.plugin_choice.selected = Some(0);
    typed.sync_watch_default();
    assert_eq!(typed.watch_exe, "launcher.exe");
    typed.plugin_choice.selected = None;
    typed.sync_watch_default();
    assert_eq!(typed.watch_exe, "launcher.exe");
    assert_eq!(typed.watch_override().as_deref(), Some("launcher.exe"));

    let mut cleared = AddDraft::new(
        PathBuf::from(r"C:\games\game.exe"),
        "Game".into(),
        plugins(),
    );
    cleared.watch_exe.clear();
    cleared.plugin_choice.selected = Some(0);
    cleared.sync_watch_default();
    assert_eq!(cleared.watch_exe, "", "a cleared field stays cleared");
}

#[test]
fn add_game_resolves_the_watch_name_the_assignment_will_use() {
    let draft = add_draft(r"C:\games\game.exe", Some(descriptor()), "");
    assert_eq!(draft.resolved_watch_name().as_deref(), Some("game.exe"));

    let draft = add_draft(
        r"C:\games\start.bat",
        Some(descriptor_watching("bin/patched_game.exe")),
        "",
    );
    assert_eq!(
        draft.resolved_watch_name().as_deref(),
        Some("patched_game.exe")
    );

    assert_eq!(
        add_draft(r"C:\games\start.bat", None, "").resolved_watch_name(),
        None
    );
}

#[test]
fn add_game_watch_override_follows_the_game_settings_rule() {
    let draft = add_draft(r"C:\games\game.exe", Some(descriptor()), " GAME.EXE ");
    assert_eq!(draft.watch_override(), None);

    let draft = add_draft(r"C:\games\game.exe", Some(descriptor()), "other.exe");
    assert_eq!(draft.watch_override().as_deref(), Some("other.exe"));

    assert_eq!(
        add_draft(r"C:\games\game.exe", Some(descriptor()), "").watch_override(),
        None
    );
    assert_eq!(
        add_draft(r"C:\games\start.bat", None, "   ").watch_override(),
        None
    );

    let draft = add_draft(r"C:\games\start.bat", None, "game.exe");
    assert_eq!(draft.watch_override().as_deref(), Some("game.exe"));
}

#[test]
fn add_game_entry_carries_only_the_override_and_leaves_the_plugin_to_the_workflow() {
    let draft = add_draft(r"C:\games\start.bat", Some(descriptor()), " other.exe ");
    let entry = draft.game_entry(GameId(9));
    assert_eq!(entry.id, GameId(9));
    assert_eq!(entry.label, "Game");
    assert_eq!(entry.selected_target, PathBuf::from(r"C:\games\start.bat"));
    assert_eq!(entry.watch_exe.as_deref(), Some("other.exe"));
    assert!(entry.plugin.is_none());
    assert!(entry.config_targets.is_empty());
    assert_eq!(entry.close_mode, CloseMode::KillExe);

    let derived = add_draft(r"C:\games\game.exe", Some(descriptor()), "game.exe");
    assert_eq!(derived.game_entry(GameId(1)).watch_exe, None);
}

#[test]
fn game_settings_keeps_a_stored_automatic_name_under_a_plugin_and_clearing_drops_it() {
    let store = |typed: &str| {
        watch_to_store(
            typed,
            Some("game.exe"),
            Some("client.exe"),
            Some("game.exe"),
        )
    };
    assert_eq!(
        store("client.exe").as_deref(),
        Some("game.exe"),
        "left as the plugin's target"
    );
    assert_eq!(store(" client.EXE ").as_deref(), Some("game.exe"));
    assert_eq!(
        store("game.exe").as_deref(),
        Some("game.exe"),
        "the automatic name as itself"
    );
    assert_eq!(store(""), None, "cleared");
    assert_eq!(
        store("launcher.exe").as_deref(),
        Some("launcher.exe"),
        "the operator's own"
    );

    assert_eq!(
        watch_to_store("client.exe", Some("game.exe"), Some("client.exe"), None),
        None
    );

    assert_eq!(
        watch_to_store("start.bat", None, None, None).as_deref(),
        Some("start.bat")
    );
}
