use super::*;
use crate::gui::common::harness::{click, locale, painted_elisions, painted_shapes, painted_texts};

fn card_buttons_filled(fill: egui::Color32) -> usize {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut actions = Vec::new();
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        controller_controls(ui, false, true, false, "Closed", &mut actions);
    });
    painted_shapes(&output)
        .into_iter()
        .filter(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == fill))
        .count()
}

#[test]
fn both_controller_card_buttons_are_primary_actions() {
    assert_eq!(
        card_buttons_filled(theme::PRIMARY_ORANGE),
        2,
        "the Open screen and Settings buttons share the primary fill"
    );
}

const SCREEN: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1080.0, 800.0));
const CONTENT: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(22.0, 560.0), egui::pos2(1058.0, 712.0));

fn launcher_context() -> egui::Context {
    let context = egui::Context::default();
    theme::apply(&context);
    theme::install_fonts(&context);
    context
}

fn frame_input(frame: u32) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(SCREEN),
        time: Some(f64::from(frame) * 0.1),
        ..Default::default()
    }
}

fn strip_frame(
    context: &egui::Context,
    input: egui::RawInput,
    config: &LauncherConfig,
    disabled: bool,
    actions: &mut Vec<UiAction>,
) -> egui::FullOutput {
    context.run_ui(input, |root| {
        root.scope_builder(egui::UiBuilder::new().max_rect(CONTENT), |ui| {
            settings(ui, config, disabled, false, "Closed", actions);
        });
    })
}

fn english_config() -> LauncherConfig {
    let mut config = LauncherConfig::default();
    config.settings.language = "en".to_owned();
    config
}

fn text_at(output: &egui::FullOutput, text: &str, outside: Option<egui::Rect>) -> egui::Rect {
    painted_texts(output)
        .into_iter()
        .find(|(painted, rect)| {
            painted == text && outside.is_none_or(|outside| !outside.contains(rect.center()))
        })
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| panic!("{text:?} is not painted"))
}

#[test]
fn each_settings_checkbox_sends_its_new_value() {
    let _locale = locale("en");
    let config = english_config();
    let mut expected: Vec<(String, UiAction)> = config_rows()
        .map(|row| {
            let action = UiAction::SetSetting(row.key, !(row.current)(&config));
            ((row.label)(), action)
        })
        .collect();
    let startup_on = crate::launcher_gui::platform::startup::is_enabled();
    expected.push((
        t!("settings.launch_on_startup").to_string(),
        UiAction::SetLaunchOnStartup(!startup_on),
    ));
    expected.push((
        t!("controller.card.enable").to_string(),
        UiAction::SetSetting(
            SettingKey::ControllerEnabled,
            !config.settings.controller_screen.enabled,
        ),
    ));
    for (label, action) in expected {
        let context = launcher_context();
        let mut actions = Vec::new();
        let mut output = None;
        for frame in 0..2 {
            output = Some(strip_frame(
                &context,
                frame_input(frame),
                &config,
                false,
                &mut actions,
            ));
        }
        assert!(actions.is_empty(), "{actions:?}");
        let target = text_at(&output.unwrap(), &label, None).center();
        let _ = strip_frame(
            &context,
            click(target, Some(SCREEN), 0.3),
            &config,
            false,
            &mut actions,
        );
        assert_eq!(actions, [action], "{label}");
    }
}

fn text_layout(output: &egui::FullOutput, text: &str) -> egui::Rect {
    painted_shapes(output)
        .into_iter()
        .find_map(|shape| match shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(egui::Rect::from_min_size(shape.pos, shape.galley.size()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} is not painted"))
}

#[test]
fn the_checkbox_rows_keep_the_strips_pitch_and_both_columns_end_together() {
    let _locale = locale("en");
    let config = english_config();
    let context = launcher_context();
    let mut actions = Vec::new();
    let mut output = None;
    for frame in 0..2 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            &config,
            false,
            &mut actions,
        ));
    }
    let output = output.unwrap();

    let icon = context.global_style().spacing.icon_width;
    let centres: Vec<f32> = painted_shapes(&output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect)
                if rect.rect.size() == egui::Vec2::splat(icon)
                    && rect.rect.left() < CONTENT.left() + 50.0 =>
            {
                Some(rect.rect.center().y)
            }
            _ => None,
        })
        .collect();
    assert_eq!(centres.len(), left_row_count(), "{centres:?}");
    for pair in centres.windows(2) {
        assert!(
            (pair[1] - pair[0] - (ROW_HEIGHT + ROW_SPACING)).abs() < 0.5,
            "{centres:?}"
        );
    }

    let caption = text_layout(&output, &t!("settings.language"));
    assert!(
        (caption.center().y - centres[0]).abs() < 0.5,
        "{caption:?} {centres:?}"
    );
    let boxes: Vec<egui::Rect> = painted_shapes(&output)
        .into_iter()
        .filter_map(|shape| match shape {
            egui::Shape::Rect(rect) if (rect.rect.width() - COMBO_WIDTH).abs() < 0.5 => {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    let last_row_bottom = centres[centres.len() - 1] + ROW_HEIGHT / 2.0;
    assert!(
        (boxes[1].bottom() - last_row_bottom).abs() < 0.5,
        "{boxes:?} {last_row_bottom}"
    );
}

fn pick(config: &LauncherConfig, closed: &str, choice: &str) -> Vec<UiAction> {
    let context = launcher_context();
    let mut actions = Vec::new();
    let mut output = None;
    for frame in 0..2 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            config,
            false,
            &mut actions,
        ));
    }
    let closed_box = text_at(&output.unwrap(), closed, None);
    let _ = strip_frame(
        &context,
        click(closed_box.center(), Some(SCREEN), 0.3),
        config,
        false,
        &mut actions,
    );

    let mut output = None;
    for frame in 4..7 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            config,
            false,
            &mut actions,
        ));
    }
    let target = text_at(&output.unwrap(), choice, Some(closed_box.expand(4.0))).center();
    let _ = strip_frame(
        &context,
        click(target, Some(SCREEN), 0.8),
        config,
        false,
        &mut actions,
    );
    actions
}

#[test]
fn the_language_combo_sends_the_picked_language_even_when_it_is_current() {
    let _locale = locale("en");
    let config = english_config();
    assert_eq!(
        pick(&config, "English", "日本語"),
        [UiAction::SetLanguage("ja")]
    );
    assert_eq!(
        pick(&config, "English", "English"),
        [UiAction::SetLanguage("en")]
    );
}

#[test]
fn re_picking_the_current_screen_sends_it_again() {
    let _locale = locale("en");
    let config = english_config();
    let main_display = t!("settings.main_display").to_string();
    assert_eq!(
        pick(&config, &main_display, &main_display),
        [UiAction::SetAdapter(None)]
    );
}

#[test]
fn a_disabled_strip_sends_nothing_and_opens_no_list() {
    let _locale = locale("en");
    let config = english_config();
    let main_display = t!("settings.main_display").to_string();
    let mut targets: Vec<String> = config_rows().map(|row| (row.label)()).collect();
    targets.push(t!("settings.launch_on_startup").to_string());
    targets.push(t!("controller.card.enable").to_string());
    targets.push("English".to_owned());
    targets.push(main_display.clone());
    let context = launcher_context();
    let mut actions = Vec::new();
    let mut output = None;
    for frame in 0..2 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            &config,
            true,
            &mut actions,
        ));
    }
    let output = output.unwrap();
    for (index, target) in targets.iter().enumerate() {
        let at = text_at(&output, target, None).center();
        let time = 0.3 + 0.2 * index as f64;
        let _ = strip_frame(
            &context,
            click(at, Some(SCREEN), time),
            &config,
            true,
            &mut actions,
        );
        let after = strip_frame(
            &context,
            frame_input(20 + 2 * index as u32),
            &config,
            true,
            &mut actions,
        );

        let texts = painted_texts(&after);
        assert!(
            texts.iter().all(|(text, _)| text != "日本語")
                && texts
                    .iter()
                    .filter(|(text, _)| *text == main_display)
                    .count()
                    == 1,
            "{target}: a disabled combo box opens no list"
        );
    }
    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn every_monitor_label_shows_in_full_in_the_screen_combo() {
    use crate::launcher_gui::monitors::MonitorInfo;
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = locale(code);
        let context = launcher_context();
        let mut frame = 0;
        for primary in [true, false] {
            let monitors: Vec<MonitorInfo> = (0..9)
                .map(|index| MonitorInfo {
                    index,
                    x: 0,
                    y: 0,
                    width: 7680,
                    height: 4320,
                    primary,
                })
                .collect();
            for adapter in std::iter::once(None).chain((0..9).map(Some)) {
                let mut config = english_config();
                config.settings.adapter = adapter;
                let mut actions = Vec::new();
                let mut output = None;
                for _ in 0..2 {
                    frame += 1;
                    output = Some(context.run_ui(frame_input(frame), |ui| {
                        screen_combo(ui, &config, &monitors, false, &mut actions);
                    }));
                }
                let elided: Vec<_> = painted_elisions(&output.unwrap())
                    .into_iter()
                    .filter(|(_, elided)| *elided)
                    .collect();
                assert!(
                    elided.is_empty(),
                    "{code}, screen {adapter:?}, primary {primary}: {elided:?}"
                );
            }
        }
    }
}

#[test]
fn the_strip_fits_the_main_window_in_every_locale() {
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = locale(code);
        let mut config = LauncherConfig::default();
        config.settings.language = code.to_owned();
        let context = launcher_context();
        let mut actions = Vec::new();
        let mut output = None;
        for frame in 0..2 {
            output = Some(strip_frame(
                &context,
                frame_input(frame),
                &config,
                false,
                &mut actions,
            ));
        }
        let right = painted_shapes(&output.unwrap())
            .iter()
            .map(|shape| shape.visual_bounding_rect().right())
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            right <= CONTENT.right() + 0.5,
            "{code}: the strip reaches {right}, past the content's right edge {}",
            CONTENT.right()
        );
    }
}

#[test]
fn a_list_left_open_when_the_launcher_turns_busy_closes_and_sends_nothing() {
    let _locale = locale("en");
    let config = english_config();
    let context = launcher_context();
    let mut actions = Vec::new();
    let mut output = None;
    for frame in 0..2 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            &config,
            false,
            &mut actions,
        ));
    }
    let closed_box = text_at(&output.unwrap(), "English", None);
    let _ = strip_frame(
        &context,
        click(closed_box.center(), Some(SCREEN), 0.3),
        &config,
        false,
        &mut actions,
    );
    let mut output = None;
    for frame in 4..7 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            &config,
            false,
            &mut actions,
        ));
    }
    let japanese = text_at(&output.unwrap(), "日本語", Some(closed_box.expand(4.0))).center();
    let mut output = None;
    for frame in 8..10 {
        output = Some(strip_frame(
            &context,
            frame_input(frame),
            &config,
            true,
            &mut actions,
        ));
    }
    assert!(
        painted_texts(&output.unwrap())
            .iter()
            .all(|(text, _)| text != "日本語"),
        "the list closes when the launcher turns busy"
    );
    let _ = strip_frame(
        &context,
        click(japanese, Some(SCREEN), 1.2),
        &config,
        true,
        &mut actions,
    );
    assert!(actions.is_empty(), "{actions:?}");
}
