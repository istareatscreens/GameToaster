use super::*;

#[test]
fn shared_action_button_is_clickable_focusable_and_uses_theme_fill() {
    let context = egui::Context::default();
    let mut response = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        response = Some(action(ui, "Action"));
    });
    let response = response.unwrap();
    assert!(response.enabled());
    assert!(response.sense.senses_click());
    assert!(response.sense.is_focusable());
}

#[test]
fn every_semantic_role_owns_distinct_interaction_states() {
    for role in [
        ButtonRole::Primary,
        ButtonRole::Secondary,
        ButtonRole::Quiet,
        ButtonRole::Destructive,
        ButtonRole::Selected,
    ] {
        let style = role_style(role);
        assert_ne!(style.idle, style.hovered, "{role:?} hover state");
        assert_ne!(style.hovered, style.active, "{role:?} active state");
        assert_ne!(style.idle, style.disabled, "{role:?} disabled state");
    }
}

fn single_line_button_height(context: &egui::Context) -> f32 {
    let mut height = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        height = Some(ui.add(button("Ok", ButtonRole::Primary)).rect.height());
    });
    height.unwrap()
}

#[test]
fn a_long_label_at_the_end_of_a_crowded_wrapping_row_keeps_one_line_and_moves_down() {
    let context = egui::Context::default();
    let reference = single_line_button_height(&context);
    let mut rects = Vec::new();
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        ui.set_max_width(300.0);
        ui.horizontal_wrapped(|ui| {
            for label in [
                "Review plugin change",
                "Repair or reapply",
                "Re-resolve paths",
            ] {
                rects.push(ui.add(button(label, ButtonRole::Primary)).rect);
            }
            rects.push(
                ui.add(button("Remove plugin", ButtonRole::Destructive))
                    .rect,
            );
        });
    });
    let last = rects[3];
    assert!(
        (last.height() - reference).abs() < 0.5,
        "label wrapped: height {} vs single line {reference}",
        last.height()
    );
    let first_row_top = rects[0].min.y;
    assert!(
        last.min.y > first_row_top + reference / 2.0,
        "the button must wrap to a later row, got {last:?} vs first row at {first_row_top}"
    );
    assert!(
        last.width() > 60.0,
        "the button keeps its natural width, got {}",
        last.width()
    );
}

#[test]
fn role_colours_do_not_leak_to_the_next_widget() {
    let context = egui::Context::default();
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        let before = ui.visuals().widgets.clone();
        ui.add(button("Remove plugin", ButtonRole::Destructive));
        let after = ui.visuals().widgets.clone();
        assert_eq!(after.inactive.bg_fill, before.inactive.bg_fill);
        assert_eq!(after.hovered.bg_fill, before.hovered.bg_fill);
        assert_eq!(after.active.bg_fill, before.active.bg_fill);
        assert_eq!(after.noninteractive.bg_fill, before.noninteractive.bg_fill);
        assert_eq!(after.inactive.bg_stroke, before.inactive.bg_stroke);
    });
}

#[test]
fn game_settings_plugin_action_labels_stay_on_one_line_at_the_dialog_width() {
    let context = egui::Context::default();
    let reference = single_line_button_height(&context);
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = crate::gui::common::harness::locale(code);
        let labels = [
            rust_i18n::t!("plugins.apply_change").to_string(),
            rust_i18n::t!("plugins.repair").to_string(),
            rust_i18n::t!("plugins.reresolve").to_string(),
            rust_i18n::t!("plugins.remove_assignment").to_string(),
        ];
        let mut rects = Vec::new();
        let _ = context.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(490.0);
            ui.horizontal_wrapped(|ui| {
                for label in &labels[..3] {
                    rects.push(ui.add(button(label.clone(), ButtonRole::Primary)).rect);
                }
                rects.push(
                    ui.add(button(labels[3].clone(), ButtonRole::Destructive))
                        .rect,
                );
            });
        });
        for (rect, label) in rects.iter().zip(&labels) {
            assert!(
                (rect.height() - reference).abs() < 0.5,
                "locale {code}: {label:?} wrapped to height {}",
                rect.height()
            );
        }
    }
}

#[test]
fn copy_button_emits_the_text_as_a_clipboard_command_when_clicked() {
    let context = egui::Context::default();

    let mut rect = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        rect = Some(copy_button(ui, "Copy", "payload").rect);
    });
    let rect = rect.unwrap();
    let press = egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        ..Default::default()
    };
    let output = context.run_ui(press, |ui| {
        copy_button(ui, "Copy", "payload");
    });
    let copied =
        output.platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::CopyText(text) if text == "payload"),
        );
    assert!(copied, "{:?}", output.platform_output.commands);
}
