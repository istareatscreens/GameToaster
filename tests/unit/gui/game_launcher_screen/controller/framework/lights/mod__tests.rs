use super::super::descriptor::ButtonBinding;
use super::*;

struct NamedPorts;

impl ControllerLights for NamedPorts {
    fn name(&self) -> &'static str {
        "named-ports"
    }

    fn validate_ports(&self, ports: &str) -> Result<(), String> {
        if ports.contains("bad") {
            Err(format!("not a port: {ports}"))
        } else {
            Ok(())
        }
    }

    fn turn_off(&self, _ports: &str) -> Result<(), LightsError> {
        Ok(())
    }
}

static NAMED_PORTS: NamedPorts = NamedPorts;

const PORTS: ControllerOptionDescriptor = ControllerOptionDescriptor {
    key: "led_ports",
    label_key: "controller.lights.ports",
    default: "",
    kind: ControllerOptionKind::Text { allow_empty: true },
};
const NO_BINDINGS: &[ButtonBinding] = &[];

fn descriptor(
    options: &'static [ControllerOptionDescriptor],
    lights: Option<LightsDescriptor>,
) -> ControllerDriverDescriptor {
    ControllerDriverDescriptor {
        id: "lit",
        label_key: "controller.type.adx_io4",
        recommended_layout: "washing-machine",
        options,
        lights,
        bindings: NO_BINDINGS,
    }
}

fn lit() -> ControllerDriverDescriptor {
    descriptor(
        &[TURN_OFF_ON_GAME_CLOSE, PORTS],
        Some(LightsDescriptor {
            ports_option: "led_ports",
            lights: &NAMED_PORTS,
        }),
    )
}

fn config(switch: &str, ports: &str) -> ControllerConfig {
    ControllerConfig::new(
        "lit",
        [(TURN_OFF_ON_GAME_CLOSE.key, switch), ("led_ports", ports)],
    )
}

#[test]
fn the_shared_switch_is_an_off_by_default_boolean() {
    assert_eq!(TURN_OFF_ON_GAME_CLOSE.key, "turn_lights_off_on_game_close");
    assert_eq!(TURN_OFF_ON_GAME_CLOSE.default, "false");
    assert_eq!(TURN_OFF_ON_GAME_CLOSE.kind, ControllerOptionKind::Boolean);
}

#[test]
fn lights_are_requested_only_with_lights_the_switch_on_and_a_port() {
    let switched_on = config("true", "COM21");
    let wanted = request(&lit(), &switched_on).expect("lights to turn off");
    assert_eq!(wanted.ports, "COM21");
    assert_eq!(wanted.lights.name(), "named-ports");

    assert!(request(&lit(), &config("false", "COM21")).is_none());
    assert!(request(&lit(), &config("true", "  ")).is_none());
    let without_lights = descriptor(&[], None);
    assert!(request(
        &without_lights,
        &ControllerConfig::new("lit", [] as [(&str, &str); 0])
    )
    .is_none());
}

#[test]
fn the_switch_needs_a_port_and_any_port_named_must_be_valid() {
    let _locale = crate::gui::common::harness::locale("en");
    let missing = lit().validate(&config("true", " ")).unwrap_err();
    assert_eq!(
        missing,
        rust_i18n::t!("controller.validation.lights_ports").to_string()
    );
    assert!(lit().validate(&config("false", "")).is_ok());
    assert!(lit().validate(&config("true", "COM21")).is_ok());
    assert_eq!(
        lit().validate(&config("false", "bad")).unwrap_err(),
        "not a port: bad"
    );
}

#[test]
fn a_driver_declares_lights_with_both_options_or_neither() {
    assert!(check_declaration(&lit()).is_ok());
    assert!(check_declaration(&descriptor(&[PORTS], None)).is_ok());

    let lights = Some(LightsDescriptor {
        ports_option: "led_ports",
        lights: &NAMED_PORTS,
    });
    let no_switch = check_declaration(&descriptor(&[PORTS], lights)).unwrap_err();
    assert!(
        no_switch.contains("has lights but no 'turn_lights_off_on_game_close'"),
        "{no_switch}"
    );
    let no_ports = check_declaration(&descriptor(&[TURN_OFF_ON_GAME_CLOSE], lights)).unwrap_err();
    assert!(
        no_ports.contains("names lights ports option 'led_ports'"),
        "{no_ports}"
    );
    let switch_only = check_declaration(&descriptor(&[TURN_OFF_ON_GAME_CLOSE], None)).unwrap_err();
    assert!(switch_only.contains("has no lights"), "{switch_only}");
}

#[test]
fn descriptors_compare_their_lights_by_option_and_kind() {
    struct Other;
    impl ControllerLights for Other {
        fn name(&self) -> &'static str {
            "other"
        }
        fn validate_ports(&self, _ports: &str) -> Result<(), String> {
            Ok(())
        }
        fn turn_off(&self, _ports: &str) -> Result<(), LightsError> {
            Ok(())
        }
    }
    static OTHER: Other = Other;
    let named = LightsDescriptor {
        ports_option: "led_ports",
        lights: &NAMED_PORTS,
    };
    assert_eq!(named, named);
    assert_ne!(
        named,
        LightsDescriptor {
            ports_option: "led_ports",
            lights: &OTHER,
        }
    );
    assert_ne!(
        named,
        LightsDescriptor {
            ports_option: "ports",
            lights: &NAMED_PORTS,
        }
    );
}
