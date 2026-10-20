use std::path::{Path, PathBuf};

use super::*;

#[test]
fn the_screen_draws_the_dialogs_choice_while_it_is_open_and_the_saved_one_otherwise() {
    let saved = ControllerScreenConfig {
        background_image: Some(PathBuf::from("launcher_assets/backgrounds/saved.png")),
        ..ControllerScreenConfig::default()
    };
    let mut draft = SettingsDraft::new(&saved, true).unwrap();
    assert_eq!(
        background_to_draw(Some(&draft), &saved),
        Some(Path::new("launcher_assets/backgrounds/saved.png")),
        "an untouched dialog shows the saved image"
    );
    draft.value.background_image = Some(PathBuf::from("C:/pictures/picked.png"));
    assert_eq!(
        background_to_draw(Some(&draft), &saved),
        Some(Path::new("C:/pictures/picked.png")),
        "a picked image shows before it is saved"
    );
    draft.value.background_image = None;
    assert_eq!(
        background_to_draw(Some(&draft), &saved),
        None,
        "Clear shows the themed background before it is saved"
    );
    assert_eq!(
        background_to_draw(None, &saved),
        Some(Path::new("launcher_assets/backgrounds/saved.png")),
        "Cancel drops the draft, so the saved image shows again"
    );
}
