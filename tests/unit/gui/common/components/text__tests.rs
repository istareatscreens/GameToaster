use super::*;

fn resolved(text: RichText) -> (f32, egui::Color32) {
    let context = egui::Context::default();
    theme::apply(&context);
    let mut job = egui::text::LayoutJob::default();
    text.append_to(
        &mut job,
        &context.global_style(),
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    let format = &job.sections[0].format;
    (format.font_id.size, format.color)
}

#[test]
fn every_text_role_owns_its_colour_and_size() {
    assert_eq!(resolved(hint("hint")), (13.0, theme::MUTED));
    assert_eq!(resolved(note("note")), (17.0, theme::MUTED));
    assert_eq!(resolved(path("C:/games")), (17.0, theme::PATH_BLUE));
    assert_eq!(resolved(path("C:/games").small()), (13.0, theme::PATH_BLUE));
    assert_eq!(resolved(status("Launching")), (17.0, theme::PATH_BLUE));
    assert_eq!(resolved(warning("Remove?")), (17.0, theme::ERROR));
}

#[test]
fn copyable_text_is_selectable_but_never_edits() {
    let context = egui::Context::default();
    let message = String::from("plugin executable: The handle is invalid");
    let mut response = None;
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        response = Some(error_text(ui, &message));
    });
    let response = response.unwrap();

    assert!(response.sense.senses_click());
    assert!(response.sense.is_focusable());
    assert_eq!(message, "plugin executable: The handle is invalid");
}
