use super::*;
use crate::gui::common::harness::locale;

#[test]
fn a_message_is_shown_in_the_interface_language_and_logged_in_english() {
    let _locale = locale("ja");
    let message = message!("errors.game_exists", id => 7);
    assert_eq!(message.shown, "ゲーム 7 は既に存在します");
    assert_eq!(message.logged, "game 7 already exists");
    assert_eq!(message.to_string(), message.shown);
    assert_eq!(String::from(message.clone()), message.shown);
}

#[test]
fn technical_detail_is_shown_and_logged_as_it_is() {
    let _locale = locale("ja");
    let detail = Message::from("DisplayConfigGetDeviceInfo failed with 0x1F".to_string());
    assert_eq!(detail.shown, "DisplayConfigGetDeviceInfo failed with 0x1F");
    assert_eq!(detail.logged, detail.shown);
}
