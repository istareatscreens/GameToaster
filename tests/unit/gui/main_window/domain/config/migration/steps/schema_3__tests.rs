use std::collections::BTreeMap;

use super::super::super::super::parse_document;
use super::*;

fn game(text: &str) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut document = parse_document(text).unwrap();
    upgrade(&mut document, &mut Findings::default())?;
    Ok(document
        .get_map_ref()
        .get("game.1")
        .map(|values| values.clone().into_iter().collect())
        .unwrap_or_default())
}

fn entry(key: &str, value: &str) -> (String, Option<String>) {
    (key.to_string(), Some(value.to_string()))
}

#[test]
fn a_script_mode_keeps_its_script_as_the_close_script() {
    for mode in ["script", "bat", " Script ", "BAT"] {
        let upgraded = game(&format!(
            "[game.1]\nlabel=Game\nclose_mode={mode}\nclose_script=C:\\遊戲\\close.bat\n"
        ))
        .unwrap();
        assert_eq!(
            upgraded,
            BTreeMap::from([
                entry("label", "Game"),
                entry("close_script", r"C:\遊戲\close.bat"),
            ]),
            "{mode:?}"
        );
    }
}

#[test]
fn a_kill_mode_drops_a_script_that_never_ran() {
    for mode in ["kill", "kill-exe", "killexe", " KILL ", ""] {
        let upgraded = game(&format!(
            "[game.1]\nlabel=Game\nclose_mode={mode}\nclose_script=C:\\old\\close.bat\n"
        ))
        .unwrap();
        assert_eq!(
            upgraded,
            BTreeMap::from([entry("label", "Game")]),
            "{mode:?}"
        );
    }

    let upgraded = game("[game.1]\nlabel=Game\nclose_script=C:\\old\\close.bat\n").unwrap();
    assert_eq!(upgraded, BTreeMap::from([entry("label", "Game")]));
}

#[test]
fn a_pre_start_script_found_in_schema_3_gains_no_meaning() {
    let upgraded = game(
        "[game.1]\nlabel=Game\nclose_mode=script\nclose_script=C:\\stop.bat\npre_start_script=C:\\start.bat\n",
    )
    .unwrap();
    assert_eq!(
        upgraded,
        BTreeMap::from([
            entry("label", "Game"),
            entry("close_script", r"C:\stop.bat")
        ])
    );
}

#[test]
fn a_mode_schema_3_never_accepted_fails_the_step() {
    let error = game("[game.1]\nlabel=Game\nclose_mode=sometimes\n")
        .expect_err("schema 3 refused this value");
    assert_eq!(error, "unknown close_mode 'sometimes' in [game.1]");
}

#[test]
fn only_game_sections_change() {
    let mut document = parse_document(
        "[launcher]\nclose_mode=script\nclose_script=C:\\x.bat\n\
         [game_launcher_screen]\nclose_script=C:\\y.bat\n\
         [game.2]\nlabel=Two\nclose_mode=kill\n",
    )
    .unwrap();
    upgrade(&mut document, &mut Findings::default()).unwrap();
    let map = document.get_map_ref();
    assert_eq!(
        map["launcher"].get("close_script"),
        Some(&Some(r"C:\x.bat".into()))
    );
    assert_eq!(
        map["launcher"].get("close_mode"),
        Some(&Some("script".into()))
    );
    assert_eq!(
        map["game_launcher_screen"].get("close_script"),
        Some(&Some(r"C:\y.bat".into()))
    );
    assert!(!map["game.2"].contains_key("close_mode"));
}
