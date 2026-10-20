use super::*;

#[test]
fn minimize_action_stays_inside_minimum_window_bounds() {
    let context = egui::Context::default();
    let mut response = None;
    let mut actions = Vec::new();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1080.0, 800.0),
        )),
        ..Default::default()
    };
    let _ = context.run_ui(input, |ui| {
        response = Some(minimize_panel(ui, &mut actions));
    });
    let rect = response.unwrap().rect;
    assert!(rect.min.y >= 0.0);
    assert!(rect.max.y <= 800.0);
    assert!(rect.height() >= MINIMIZE_HEIGHT);
}

fn settings_block(context: &egui::Context) -> (f32, f32) {
    let config = LauncherConfig::default();
    let mut actions = Vec::new();
    let mut measured = None;

    for frame in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1080.0, 800.0),
            )),
            time: Some(f64::from(frame) * 0.1),
            ..Default::default()
        };
        let _ = context.run_ui(input, |ui| {
            let reserved = settings::reserved_height(ui);
            let block = ui.vertical(|ui| {
                settings(ui, &config, false, false, "status", &mut actions);
            });
            measured = Some((block.response.rect.height(), reserved));
        });
    }
    measured.unwrap()
}

fn launcher_context() -> egui::Context {
    let context = egui::Context::default();
    theme::apply(&context);
    theme::install_fonts(&context);
    context
}

#[test]
fn settings_reserve_covers_the_drawn_settings_block_in_every_locale() {
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = crate::gui::common::harness::locale(code);
        let (drawn, reserved) = settings_block(&launcher_context());
        assert!(drawn > 0.0, "locale {code}: nothing was drawn");
        assert!(
            drawn <= reserved,
            "locale {code}: drew {drawn} px into a {reserved} px reserve"
        );
        assert!(
            reserved - drawn <= 12.5,
            "locale {code}: reserved {reserved} px for a {drawn} px block"
        );
    }
}

#[test]
fn the_reserve_follows_the_combo_boxes_real_height() {
    let _locale = crate::gui::common::harness::locale("en");
    let (normal, _) = settings_block(&launcher_context());
    let context = launcher_context();
    let mut style = (*context.global_style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(2.0 * theme::font_size::BODY),
    );
    context.set_global_style(style);
    let (drawn, reserved) = settings_block(&context);
    assert!(
        drawn > normal,
        "the larger combo boxes make the block taller: {drawn} against {normal}"
    );
    assert!(
        drawn <= reserved,
        "drew {drawn} px into a {reserved} px reserve"
    );
}

#[test]
fn the_reserve_tracks_the_row_count_and_the_credit_toggle_is_not_drawn() {
    assert_eq!(settings::left_row_count(), 5);
    let rows = settings::left_row_count() as f32;
    let context = launcher_context();
    let mut reserved = 0.0;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        reserved = settings::reserved_height(ui);
    });
    assert!(reserved >= rows * 22.0 + (rows - 1.0) * 6.0);
    assert!(!settings::draws_setting(SettingKey::BlockExitOnCredit));
    for key in [
        SettingKey::AutoHide,
        SettingKey::HideCommandWindows,
        SettingKey::LaunchDefaultOnOpen,
        SettingKey::PauseOnFailure,
    ] {
        assert!(settings::draws_setting(key), "{key:?} must be drawn");
    }
}
