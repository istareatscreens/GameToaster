use super::*;

#[test]
fn safe_defaults_are_opt_in_registry_ids() {
    let value = ControllerScreenConfig::default();
    assert!(!value.launch_on_start);
    assert!(!value.controller_only_start);
    assert!(!value.reopen_on_monitor_return);
    assert_eq!(value.controller.driver_id, "adx-io4");
    assert_eq!(value.controller.options.get("vid").unwrap(), "0CA3");
    assert_eq!(value.controller.options.get("pid").unwrap(), "0021");
    assert_eq!(value.controller.options.get("led_ports").unwrap(), "COM21");
    assert_eq!(
        value
            .controller
            .options
            .get("turn_lights_off_on_game_close")
            .unwrap(),
        "false"
    );
    assert_eq!(value.launcher_layout, "washing-machine");
    crate::game_launcher_screen::validate_screen_config(&value).unwrap();
}

#[test]
fn validates_startup_driver_and_calibration_invariants() {
    let _locale = crate::gui::common::harness::locale("en");
    let mut value = ControllerScreenConfig {
        controller_only_start: true,
        ..Default::default()
    };
    assert!(value.validate().unwrap_err().contains("launch-on-start"));
    value.launch_on_start = true;
    value.controller = crate::game_launcher_screen::default_controller_config("jvs").unwrap();
    value
        .controller
        .options
        .insert("endpoint".into(), "not-a-port".into());
    assert!(crate::game_launcher_screen::validate_screen_config(&value)
        .unwrap_err()
        .contains("endpoint"));
    value
        .controller
        .options
        .insert("endpoint".into(), r"\\.\mxjvs".into());
    crate::game_launcher_screen::validate_screen_config(&value).unwrap();
}

#[test]
fn a_jvs_endpoint_split_inside_a_character_is_refused_not_a_crash() {
    let _locale = crate::gui::common::harness::locale("en");
    let mut config = crate::game_launcher_screen::default_controller_config("jvs").unwrap();

    for endpoint in ["ÖÖ", "a€", "COMÖ"] {
        config.options.insert("endpoint".into(), endpoint.into());
        assert!(
            crate::game_launcher_screen::validate_controller_config(&config)
                .unwrap_err()
                .contains("endpoint"),
            "{endpoint}"
        );
    }
}

#[test]
fn unknown_driver_layout_and_option_are_rejected() {
    let _locale = crate::gui::common::harness::locale("en");
    let mut config = ControllerScreenConfig::default();
    config.controller.driver_id = "missing-driver".into();
    assert!(crate::game_launcher_screen::validate_screen_config(&config)
        .unwrap_err()
        .contains("not registered"));

    config.controller = ControllerScreenConfig::default().controller;
    config
        .controller
        .options
        .insert("legacy".into(), "true".into());
    assert!(crate::game_launcher_screen::validate_screen_config(&config)
        .unwrap_err()
        .contains("does not define"));

    config.controller = ControllerScreenConfig::default().controller;
    config.launcher_layout = "missing-layout".into();
    assert!(crate::game_launcher_screen::validate_screen_config(&config)
        .unwrap_err()
        .contains("not registered"));
}

#[test]
fn startup_visibility_requires_enabled_and_both_controller_flags() {
    let mut config = ControllerScreenConfig::default();
    assert!(!config.starts_main_hidden());
    config.controller_only_start = true;
    assert!(!config.starts_main_hidden());
    assert!(config.validate().is_err());
    config.launch_on_start = true;
    assert!(!config.starts_main_hidden());
    config.enabled = true;
    assert!(config.starts_main_hidden());
    assert!(config.validate().is_ok());
    config.controller_only_start = false;
    assert!(!config.starts_main_hidden());
}
