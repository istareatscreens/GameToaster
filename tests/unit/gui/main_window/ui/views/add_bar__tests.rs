use super::*;

fn render(
    context: &egui::Context,
    input: egui::RawInput,
    busy: Option<&str>,
) -> (AddBarResponses, Vec<UiAction>) {
    let mut responses = None;
    let mut actions = Vec::new();
    let _ = context.run_ui(input, |ui| {
        responses = Some(add_game_bar(ui, busy, &mut actions));
    });
    (responses.unwrap(), actions)
}

#[test]
fn primary_buttons_are_focusable_and_keyboard_activation_routes_plugins() {
    let context = egui::Context::default();
    let (first, actions) = render(&context, egui::RawInput::default(), None);
    assert!(actions.is_empty());
    for response in [&first.add_game, &first.plugins, &first.hotkeys] {
        assert!(response.enabled());
        assert!(response.sense.senses_click());
        assert!(response.sense.is_focusable());
    }

    first.plugins.request_focus();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: Some(egui::Key::Enter),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let (_, actions) = render(&context, input, None);
    assert_eq!(actions, vec![UiAction::Plugins]);

    let (disabled, actions) = render(&context, egui::RawInput::default(), Some("busy"));
    assert!(actions.is_empty());
    assert!(!disabled.add_game.enabled());
    assert!(!disabled.plugins.enabled());
    assert!(!disabled.hotkeys.enabled());
}
