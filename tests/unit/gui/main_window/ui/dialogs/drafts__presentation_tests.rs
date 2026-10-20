use super::*;
use crate::launcher_gui::model::CloseMode;

fn game() -> GameEntry {
    GameEntry {
        id: GameId(7),
        label: "Original".into(),
        controller_image: Some("launcher_assets/game_images/7.png".into()),
        selected_target: r"C:\games\game.exe".into(),
        plugin: None,
        hotkey: None,
        watch_exe: None,
        close_mode: CloseMode::KillExe,
        close_script: None,
        close_hotkey: None,
        config_targets: Vec::new(),
    }
}

#[test]
fn presentation_choose_replace_clear_and_submission_states_are_explicit() {
    let context = eframe::egui::Context::default();
    let texture = || {
        context.load_texture(
            "presentation-test",
            eframe::egui::ColorImage::new([1, 1], vec![eframe::egui::Color32::WHITE]),
            eframe::egui::TextureOptions::LINEAR,
        )
    };
    let mut draft = PresentationDraft::new(game(), None, Some("old error".into()));
    draft.select_image("first.jpg".into(), texture());
    assert_eq!(
        draft.selected_source.as_deref(),
        Some(std::path::Path::new("first.jpg"))
    );
    draft.select_image("replacement.png".into(), texture());
    draft.label = "  Renamed  ".into();
    assert_eq!(
        draft.submission().unwrap(),
        ("Renamed".into(), Some("replacement.png".into()), false)
    );
    draft.clear_image();
    assert_eq!(draft.submission().unwrap(), ("Renamed".into(), None, true));
    draft.label = "   ".into();
    assert!(draft.submission().is_err());
}
