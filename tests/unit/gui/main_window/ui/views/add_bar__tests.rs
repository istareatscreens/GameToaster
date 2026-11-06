use super::*;

fn render(
    context: &egui::Context,
    input: egui::RawInput,
    busy: Option<&str>,
) -> (AddBarResponses, Vec<UiAction>) {
    render_launch(context, input, busy, false)
}

fn render_launch(
    context: &egui::Context,
    input: egui::RawInput,
    busy: Option<&str>,
    launch_cancellable: bool,
) -> (AddBarResponses, Vec<UiAction>) {
    let mut responses = None;
    let mut actions = Vec::new();
    let _ = context.run_ui(input, |ui| {
        responses = Some(add_game_bar(ui, busy, launch_cancellable, &mut actions));
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

#[test]
fn cancel_launch_appears_only_while_a_launch_can_be_cancelled() {
    let context = egui::Context::default();
    let (idle, _) = render(&context, egui::RawInput::default(), None);
    assert!(idle.cancel_launch.is_none());

    let (saving, _) = render(&context, egui::RawInput::default(), Some("Saving"));
    assert!(saving.cancel_launch.is_none());
}

#[test]
fn cancel_launch_stays_usable_while_the_launch_disables_everything_else() {
    let context = egui::Context::default();
    let (bar, actions) = render_launch(
        &context,
        egui::RawInput::default(),
        Some("Running the pre-start script"),
        true,
    );
    assert!(actions.is_empty());
    assert!(!bar.add_game.enabled());
    let cancel = bar.cancel_launch.expect("the cancel is offered");
    assert!(cancel.enabled());
    assert!(cancel.sense.senses_click());
    assert!(cancel.sense.is_focusable());

    cancel.request_focus();
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
    let (_, actions) = render_launch(&context, input, Some("Running the pre-start script"), true);
    assert_eq!(actions, vec![UiAction::CancelLaunch]);
}

#[test]
fn a_long_status_is_cut_to_the_room_left_and_never_widens_the_bar() {
    let context = egui::Context::default();
    let width = 700.0;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 400.0),
        )),
        ..Default::default()
    };
    let long = "The game process \"never_appears.exe\" did not start in time. The launcher stayed \
                open so you can check the game window.";
    let mut right = 0.0_f32;
    let _ = context.run_ui(input, |ui| {
        let mut actions = Vec::new();
        add_game_bar(ui, Some(long), true, &mut actions).discard();
        right = ui.min_rect().right();
    });
    assert!(
        right <= width + 0.5,
        "the bar ends at {right}, past the window's {width}"
    );
}
