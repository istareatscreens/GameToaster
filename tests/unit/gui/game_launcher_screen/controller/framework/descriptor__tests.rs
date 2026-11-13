use super::*;

static BOUND: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "bound",
    label_key: "controller.type.adx_io4",
    recommended_layout: "washing-machine",
    options: &[],
    lights: None,
    bindings: &[
        ButtonBinding {
            button: Button::Ba7,
            command: LauncherCommand::Previous,
        },
        ButtonBinding {
            button: Button::Select,
            command: LauncherCommand::Launch,
        },
    ],
};

#[test]
fn command_for_returns_only_the_bound_command() {
    assert_eq!(
        BOUND.command_for(Button::Ba7),
        Some(LauncherCommand::Previous)
    );
    assert_eq!(
        BOUND.command_for(Button::Select),
        Some(LauncherCommand::Launch)
    );
    for button in Button::ALL {
        if button != Button::Ba7 && button != Button::Select {
            assert_eq!(BOUND.command_for(button), None, "{button:?}");
        }
    }
}

static RING: ControllerDriverDescriptor = ControllerDriverDescriptor {
    id: "ring",
    label_key: "controller.type.adx_io4",
    recommended_layout: "washing-machine",
    options: &[
        ControllerOptionDescriptor {
            key: "vid",
            label_key: "controller.type.vid",
            default: "0CA3",
            kind: ControllerOptionKind::HexU16 { non_zero: true },
        },
        ControllerOptionDescriptor {
            key: "pid",
            label_key: "controller.type.pid",
            default: "0021",
            kind: ControllerOptionKind::HexU16 { non_zero: true },
        },
        ControllerOptionDescriptor {
            key: "note",
            label_key: "controller.type.vid",
            default: "",
            kind: ControllerOptionKind::Text { allow_empty: true },
        },
    ],
    lights: None,
    bindings: &[],
};

fn ring(vid: &str, pid: &str) -> ControllerConfig {
    ControllerConfig::new("ring", [("vid", vid), ("pid", pid), ("note", "")])
}

#[test]
fn an_emptied_identity_field_is_the_drivers_own() {
    for emptied in ["", "   "] {
        let config = ring(emptied, "ABCD");
        assert_eq!(RING.validate(&config), Ok(()), "{emptied:?}");
        assert_eq!(RING.hex_u16(&config, "vid"), Ok(0x0CA3), "{emptied:?}");
        assert_eq!(RING.hex_u16(&config, "pid"), Ok(0xABCD), "{emptied:?}");

        let config = ring("ABCD", emptied);
        assert_eq!(RING.validate(&config), Ok(()), "{emptied:?}");
        assert_eq!(RING.hex_u16(&config, "vid"), Ok(0xABCD), "{emptied:?}");
        assert_eq!(RING.hex_u16(&config, "pid"), Ok(0x0021), "{emptied:?}");
    }

    let config = ring("", "");
    assert_eq!(RING.validate(&config), Ok(()));
    assert_eq!(RING.hex_u16(&config, "vid"), Ok(0x0CA3));
    assert_eq!(RING.hex_u16(&config, "pid"), Ok(0x0021));
}

#[test]
fn an_entered_identity_field_replaces_the_drivers_own() {
    let config = ring(" 2e3c ", "00005750");
    assert_eq!(RING.validate(&config), Ok(()));
    assert_eq!(RING.hex_u16(&config, "vid"), Ok(0x2E3C));
    assert_eq!(RING.hex_u16(&config, "pid"), Ok(0x5750));

    let config = RING.default_config();
    assert_eq!(RING.hex_u16(&config, "vid"), Ok(0x0CA3));
    assert_eq!(RING.hex_u16(&config, "pid"), Ok(0x0021));
}

#[test]
fn a_field_that_is_not_a_non_zero_hexadecimal_number_is_refused() {
    let _locale = crate::gui::common::harness::locale("en");
    for refused in [
        "not-hex",
        "0x2E3C",
        "+2E3C",
        "-1",
        "2E3C 5750",
        "12345",
        "G000",
    ] {
        let config = ring(refused, "0021");
        let error = RING.validate(&config).unwrap_err();
        assert!(
            error.contains("'vid'") && error.contains("hexadecimal"),
            "{refused}: {error}"
        );
        assert_eq!(RING.hex_u16(&config, "vid"), Err(error), "{refused}");

        assert_eq!(RING.hex_u16(&config, "pid"), Ok(0x0021), "{refused}");
    }
    for zero in ["0", "0000"] {
        let config = ring("0CA3", zero);
        let error = RING.validate(&config).unwrap_err();
        assert!(
            error.contains("'pid'") && error.contains("non-zero"),
            "{zero}: {error}"
        );
        assert_eq!(RING.hex_u16(&config, "pid"), Err(error), "{zero}");
    }
}

#[test]
fn only_a_hexadecimal_option_the_driver_defines_has_a_value() {
    let config = ring("0CA3", "0021");
    assert!(RING
        .hex_u16(&config, "serial")
        .unwrap_err()
        .contains("does not define option 'serial'"));
    assert!(RING
        .hex_u16(&config, "note")
        .unwrap_err()
        .contains("not a hexadecimal value"));

    let mut without = config;
    without.options.remove("vid");
    assert!(RING.validate(&without).is_err());
    assert!(RING
        .hex_u16(&without, "vid")
        .unwrap_err()
        .contains("'vid' is missing"));
}
