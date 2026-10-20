use std::collections::HashMap;

use super::super::super::super::parse_document;
use super::super::super::fixtures::prepared_kind;
use super::*;

fn upgraded(text: &str) -> Result<(Ini, Findings), String> {
    let mut document = parse_document(text).unwrap();
    let mut findings = Findings::default();
    upgrade(&mut document, &mut findings)?;
    Ok((document, findings))
}

fn section<'a>(document: &'a Ini, name: &str) -> &'a HashMap<String, Option<String>> {
    document
        .get_map_ref()
        .get(name)
        .unwrap_or_else(|| panic!("[{name}] is missing"))
}

#[test]
fn the_controller_section_and_the_game_keys_take_their_schema_2_names() {
    let (document, _) = upgraded(
        "[controller_screen]\nenabled=true\ncontroller_adapter=jvs\n\
         [game.1]\nlabel=Game\nexe_path=C:\\Games\\game.exe\nkind=other\nclose_mode=kill\n",
    )
    .unwrap();

    assert!(!document.get_map_ref().contains_key("controller_screen"));
    let controller = section(&document, "game_launcher_screen");
    assert_eq!(controller.get("enabled"), Some(&Some("true".into())));
    assert_eq!(
        controller.get("controller_adapter"),
        Some(&Some("jvs".into()))
    );
    let game = section(&document, "game.1");
    assert_eq!(
        game.get("selected_target"),
        Some(&Some(r"C:\Games\game.exe".into()))
    );
    assert!(!game.contains_key("exe_path") && !game.contains_key("kind"));
    assert_eq!(game.get("close_mode"), Some(&Some("kill".into())));
}

#[test]
fn prepared_games_are_listed_in_id_order_under_either_spelling() {
    let written = prepared_kind();
    let accepted = written.replace('-', "");
    let (_, findings) = upgraded(&format!(
        "[game.10]\nlabel=Ten\nexe_path=C:\\ten.exe\nkind={accepted}\n\
         [game.2]\nlabel= Two \nexe_path=C:\\two.exe\nkind= {} \n\
         [game.3]\nlabel=Three\nexe_path=C:\\three.exe\nkind=other\n",
        written.to_ascii_uppercase()
    ))
    .unwrap();
    assert_eq!(findings.plugin_needed, ["Two", "Ten"]);
}

#[test]
fn an_unknown_or_missing_game_kind_is_refused() {
    for (kind, expected) in [
        ("kind=retro\n", "unknown game kind 'retro' in [game.1]"),
        ("kind= \n", "missing or empty kind in [game.1]"),
        ("", "missing or empty kind in [game.1]"),
    ] {
        let error = upgraded(&format!(
            "[game.1]\nlabel=Game\nexe_path=C:\\game.exe\n{kind}"
        ))
        .expect_err("the kind is refused");
        assert_eq!(error, expected, "{kind:?}");
    }
}

#[test]
fn a_missing_or_empty_target_is_refused_in_the_previous_launchers_words() {
    for target in ["", "exe_path=\n", "exe_path= \n"] {
        let error = upgraded(&format!("[game.1]\nlabel=Game\n{target}kind=other\n"))
            .expect_err("the target is refused");
        assert_eq!(error, "missing or empty exe_path in [game.1]", "{target:?}");
    }
}

#[test]
fn keys_the_previous_launcher_never_read_are_dropped() {
    let (document, _) = upgraded(
        "[game_launcher_screen]\ncontroller_driver=jvs\n\
         [game.1]\nlabel=Game\nexe_path=C:\\game.exe\nkind=other\n\
         selected_target=C:\\other.exe\nplugin_id=example.plugin\n",
    )
    .unwrap();
    assert!(!document.get_map_ref().contains_key("game_launcher_screen"));
    let game = section(&document, "game.1");
    assert_eq!(
        game.get("selected_target"),
        Some(&Some(r"C:\game.exe".into()))
    );
    assert!(!game.contains_key("plugin_id"));
}
