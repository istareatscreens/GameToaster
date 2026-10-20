use super::super::harness;
use super::super::Dialog;
use super::*;
use crate::gui::common::harness::{assert_pinned, locale};
use crate::launcher_gui::model::GameId;

#[test]
fn hotkeys_offers_cancel_then_save_and_save_sends_every_binding() {
    let launcher = Hotkey {
        ctrl: true,
        alt: false,
        shift: false,
        win: false,
        key: 0x70,
    };
    let launcher_close = Hotkey {
        key: 0x71,
        ..launcher
    };
    let launch = Hotkey {
        key: 0x72,
        ..launcher
    };
    let draft = HotkeyDraft {
        launcher: Some(launcher),
        launcher_close: Some(launcher_close),
        games: vec![super::super::HotkeyRow {
            id: GameId(7),
            label: "Game".into(),
            launch: Some(launch),
            close: None,
        }],
        capturing: None,
        error: None,
    };
    let mut shown = harness::show(Dialog::Hotkeys(draft));
    assert_eq!(shown.actions().len(), 2);
    assert!(matches!(shown.click_action(0), DialogEvent::Cancel));
    match shown.click_action(1) {
        DialogEvent::SaveHotkeys {
            launcher: saved_launcher,
            launcher_close: saved_close,
            games,
        } => {
            assert_eq!(saved_launcher, Some(launcher));
            assert_eq!(saved_close, Some(launcher_close));
            assert_eq!(games, vec![(GameId(7), Some(launch), None)]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_hotkey_error_is_pinned_under_the_title() {
    let _locale = locale("en");
    let draft = HotkeyDraft {
        error: Some("that hotkey is taken".into()),
        ..HotkeyDraft::default()
    };
    let shown = harness::show(Dialog::Hotkeys(draft));
    assert_pinned(
        &shown.output,
        shown.window(),
        &t!("title.hotkeys"),
        "that hotkey is taken",
    );
}
