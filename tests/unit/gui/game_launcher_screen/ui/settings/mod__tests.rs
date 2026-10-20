use super::*;
use crate::game_launcher_screen::display::geometry::PixelRect;
use crate::game_launcher_screen::display::monitor::StableMonitorId;
use crate::gui::common::theme;

fn monitor() -> MonitorDescriptor {
    MonitorDescriptor {
        index: 0,
        id: StableMonitorId {
            device_name: r"\\.\DISPLAY1".into(),
            device_path: "display-path-1".into(),
            manufacturer: Some("SEG".into()),
            product_code: Some(1),
            serial: Some(2),
        },
        friendly_name: "Test display".into(),
        bounds: PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        rotation_degrees: 0,
        primary: true,
        dpi_x: 144,
        dpi_y: 144,
        physical_width_mm: Some(890),
        physical_height_mm: Some(500),
        native_pixels: true,
    }
}

fn draft() -> SettingsDraft {
    let monitor = monitor();
    let drivers = AdapterRegistry::with_default_adapters();
    let layouts = LauncherLayoutRegistry::with_builtin_layouts();
    SettingsDraft {
        value: ControllerScreenConfig::default(),
        monitors: vec![monitor],
        selected_monitor: Some(0),
        diameter_px: 1080,
        preview: false,
        drivers: drivers.descriptors(),
        layouts: layouts.descriptors(),
        test_status: TestStatus::NotTested,
        live_buttons: LiveButtons::NoReports,
        testing: false,
        error: None,
        startup_enabled: true,
        background_preview: BackgroundPreview::default(),
    }
}

#[test]
fn calibration_records_without_a_confirmation_gate_and_enforces_startup_dependency() {
    let _locale = crate::gui::common::harness::locale("en");
    let mut draft = draft();

    let saved = draft.selected_config().unwrap();
    assert_eq!(saved.calibration.unwrap().diameter_px, 1080);

    draft.value.controller_only_start = true;
    assert!(draft
        .selected_config()
        .unwrap_err()
        .contains("launch-on-start"));
}

#[test]
fn descriptor_owned_connection_fields_validate_without_hidden_defaults() {
    let _locale = crate::gui::common::harness::locale("en");
    let mut draft = draft();
    draft
        .value
        .controller
        .options
        .insert("vid".into(), "not-hex".into());
    assert!(draft
        .controller_config()
        .unwrap_err()
        .contains("hexadecimal"));
    draft.value.controller = crate::game_launcher_screen::default_controller_config("jvs").unwrap();
    assert!(draft.controller_config().is_ok());
    draft
        .value
        .controller
        .options
        .insert("endpoint".into(), "ttyUSB0".into());
    assert!(draft.controller_config().unwrap_err().contains("endpoint"));
}

#[test]
fn discovered_driver_config_and_recommended_layout_apply_atomically() {
    let mut draft = draft();
    let found = crate::game_launcher_screen::controller::DiscoveredController {
        config: crate::game_launcher_screen::default_controller_config("adx-hid").unwrap(),
        label_key: "controller.type.adx_hid",
    };
    draft.apply_discovery(&found);

    assert_eq!(draft.value.controller.driver_id, "adx-hid");
    assert_eq!(draft.value.launcher_layout, "washing-machine");
    assert!(matches!(draft.test_status, TestStatus::Detected(_)));
}

#[test]
fn the_settings_window_fits_the_main_window_and_scrolls_with_a_visible_handle() {
    use crate::gui::common::harness::{settled_modal, MAIN_WINDOW};
    let mut draft = draft();
    let launcher_dir = std::env::temp_dir();
    let modal = settled_modal(egui::Id::new(SETTINGS_ID), |context| {
        let _ = show(context, &launcher_dir, &mut draft);
    });
    assert_eq!(modal.bars.len(), 2, "{:?}", modal.bars);
    let ((track_rect, track), (_, handle)) = (modal.bars[0], modal.bars[1]);
    assert!((track_rect.height() - 624.0).abs() < 0.01, "{track_rect:?}");
    assert_eq!((track, handle), (theme::ICON_TILE, theme::SCROLL_HANDLE));
    let main_window = egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW);
    assert!(
        main_window.contains_rect(modal.window),
        "{:?}",
        modal.window
    );
}

#[test]
fn a_failed_save_pins_its_error_and_cancel_closes_the_dialog() {
    use crate::gui::common::harness::{
        assert_pinned, click, close_cross, footer_actions, locale, settled, MAIN_WINDOW,
    };
    let _locale = locale("en");
    let mut draft = draft();
    draft.selected_monitor = None;
    let launcher_dir = std::env::temp_dir();
    let context = egui::Context::default();
    theme::apply(&context);
    let screen = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW));
    let output = settled(&context, screen, |ui| {
        let _ = show(ui.ctx(), &launcher_dir, &mut draft);
    });
    let window = || {
        context
            .memory(|memory| memory.area_rect(egui::Id::new(SETTINGS_ID)))
            .expect("the dialog is shown")
    };
    let actions = footer_actions(&output, window());
    assert_eq!(actions.len(), 2, "{actions:?}");

    let mut action = SettingsAction::None;
    let _ = context.run_ui(click(actions[1].center(), screen, 1.2), |ui| {
        action = show(ui.ctx(), &launcher_dir, &mut draft);
    });
    assert!(
        matches!(action, SettingsAction::None),
        "a failed save saves nothing"
    );
    let error = draft
        .error
        .clone()
        .expect("the failed save records its error");
    let mut output = None;
    for frame in 13..16 {
        let input = egui::RawInput {
            screen_rect: screen,
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        output = Some(context.run_ui(input, |ui| {
            let _ = show(ui.ctx(), &launcher_dir, &mut draft);
        }));
    }
    let output = output.unwrap();
    assert_pinned(&output, window(), &t!("controller.title"), &error);

    assert!(
        close_cross(&output).is_none(),
        "the dialog has no title-bar close button"
    );
    let actions = footer_actions(&output, window());
    let _ = context.run_ui(click(actions[0].center(), screen, 1.6), |ui| {
        action = show(ui.ctx(), &launcher_dir, &mut draft);
    });
    assert!(matches!(action, SettingsAction::Cancel), "Cancel cancels");
}

#[test]
fn the_settings_dialog_keeps_the_scaffolds_width_whatever_its_names() {
    use crate::gui::common::harness::{locale, scaffold_width, settled, MAIN_WINDOW};
    let expected = scaffold_width();
    let mut wider = Vec::new();
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = locale(code);
        let mut draft = draft();
        draft.monitors[0].friendly_name = "Generic PnP Monitor on the arcade cabinet".into();
        draft.value.background_image = Some(std::path::PathBuf::from(
            "C:/Backgrounds/a_background_image_with_a_long_descriptive_file_name.png",
        ));
        let launcher_dir = std::env::temp_dir();
        let context = egui::Context::default();
        theme::apply(&context);
        theme::install_fonts(&context);
        let screen = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, MAIN_WINDOW));
        let _ = settled(&context, screen, |ui| {
            let _ = show(ui.ctx(), &launcher_dir, &mut draft);
        });
        let width = context
            .memory(|memory| memory.area_rect(egui::Id::new(SETTINGS_ID)))
            .expect("the dialog is shown")
            .width();
        if (width - expected).abs() >= 0.5 {
            wider.push(format!("{code}: {width}"));
        }
    }
    assert!(
        wider.is_empty(),
        "the scaffold's width is {expected}: {wider:?}"
    );
}
