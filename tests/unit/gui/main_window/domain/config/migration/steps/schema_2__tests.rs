use std::collections::BTreeMap;

use super::super::super::super::parse_document;
use super::*;

fn controller(text: &str) -> Result<BTreeMap<String, Option<String>>, String> {
    let mut document = parse_document(text).unwrap();
    upgrade(&mut document, &mut Findings::default())?;
    Ok(document
        .get_map_ref()
        .get(CONTROLLER_SECTION)
        .map(|values| values.clone().into_iter().collect())
        .unwrap_or_default())
}

fn entry(key: &str, value: &str) -> (String, Option<String>) {
    (key.to_string(), Some(value.to_string()))
}

#[test]
fn the_adapter_becomes_the_driver_and_io4_is_adx_io4() {
    for (adapter, driver) in [
        ("adx-io4", "adx-io4"),
        ("io4", "adx-io4"),
        (" IO4 ", "adx-io4"),
        ("adx-hid", "adx-hid"),
        ("JVS", "jvs"),
    ] {
        let upgraded = controller(&format!(
            "[game_launcher_screen]\ncontroller_adapter={adapter}\n"
        ))
        .unwrap();
        assert_eq!(
            upgraded,
            BTreeMap::from([entry("controller_driver", driver)]),
            "{adapter}"
        );
    }
}

#[test]
fn adx_ids_lose_their_prefix_and_keep_four_hexadecimal_digits() {
    let upgraded =
        controller("[game_launcher_screen]\ncontroller_adapter=adx-hid\nvid=0x2e3c\npid=21\n")
            .unwrap();
    assert_eq!(
        upgraded,
        BTreeMap::from([
            entry("controller_driver", "adx-hid"),
            entry("controller_option.vid", "2E3C"),
            entry("controller_option.pid", "0021"),
        ])
    );
    let upper = controller("[game_launcher_screen]\nvid=0XABCD\n").unwrap();
    assert_eq!(
        upper.get("controller_option.vid"),
        Some(&Some("ABCD".into()))
    );
}

#[test]
fn the_jvs_player_and_endpoint_become_options() {
    for (player, option) in [("p1", "p1"), ("1", "p1"), (" P2 ", "p2"), ("2", "p2")] {
        let upgraded = controller(&format!(
            "[game_launcher_screen]\ncontroller_adapter=jvs\njvs_player={player}\njvs_endpoint= COM7 \n"
        ))
        .unwrap();
        assert_eq!(
            upgraded,
            BTreeMap::from([
                entry("controller_driver", "jvs"),
                entry("controller_option.player", option),
                entry("controller_option.endpoint", "COM7"),
            ]),
            "{player}"
        );
    }
}

#[test]
fn missing_or_empty_values_leave_the_drivers_defaults() {
    let upgraded = controller(
        "[game_launcher_screen]\nenabled=true\ncontroller_adapter=\nvid=\npid\njvs_player=\n",
    )
    .unwrap();
    assert_eq!(
        upgraded,
        BTreeMap::from([
            entry("enabled", "true"),
            entry("controller_driver", "adx-io4"),
        ])
    );
}

#[test]
fn the_other_drivers_keys_are_dropped_unread() {
    let adx = controller(
        "[game_launcher_screen]\ncontroller_adapter=adx-io4\njvs_player=seven\n\
         jvs_endpoint=nowhere\njvs_hardware_validated=maybe\n",
    )
    .unwrap();
    assert_eq!(adx, BTreeMap::from([entry("controller_driver", "adx-io4")]));
    let jvs =
        controller("[game_launcher_screen]\ncontroller_adapter=jvs\nvid=zz\npid=0x0000\n").unwrap();
    assert_eq!(jvs, BTreeMap::from([entry("controller_driver", "jvs")]));
}

#[test]
fn keys_schema_2_never_read_are_dropped() {
    let upgraded = controller(
        "[game_launcher_screen]\ncontroller_adapter=adx-hid\ncontroller_driver=jvs\n\
         controller_option.player=p2\nlauncher_layout=compact\nreopen_on_monitor_return=true\n",
    )
    .unwrap();
    assert_eq!(
        upgraded,
        BTreeMap::from([entry("controller_driver", "adx-hid")])
    );
}

#[test]
fn an_unknown_adapter_player_or_id_is_refused() {
    for (text, expected) in [
        (
            "controller_adapter=keyboard\n",
            "unknown controller adapter 'keyboard'",
        ),
        (
            "controller_adapter=jvs\njvs_player=p3\n",
            "unknown JVS player profile 'p3'",
        ),
        (
            "controller_adapter=adx-hid\nvid=0xZZ\n",
            "invalid [game_launcher_screen] vid='0xZZ': expected 16-bit hexadecimal",
        ),
        (
            "pid=12345\n",
            "invalid [game_launcher_screen] pid='12345': expected 16-bit hexadecimal",
        ),
    ] {
        let error = controller(&format!("[game_launcher_screen]\n{text}")).expect_err(text);
        assert_eq!(error, expected);
    }
}

#[test]
fn a_file_without_the_controller_section_keeps_the_defaults() {
    let mut document = parse_document("[launcher]\nschema_version=2\n").unwrap();
    upgrade(&mut document, &mut Findings::default()).unwrap();
    assert!(!document.get_map_ref().contains_key(CONTROLLER_SECTION));
}
