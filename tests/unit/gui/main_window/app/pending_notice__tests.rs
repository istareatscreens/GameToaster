use std::path::PathBuf;

use super::*;

fn notice() -> Option<Notice> {
    Some(Notice::Upgraded {
        kept: PathBuf::from(r"C:\State\launcherConfig.ini"),
        plugin_needed: Vec::new(),
    })
}

#[test]
fn a_free_dialog_shows_the_notice_and_it_stays_pending_until_closed() {
    let mut dialog = None;
    let pending = notice();

    show_when_free(&mut dialog, &pending);

    assert!(matches!(dialog, Some(Dialog::Notice { .. })));
    assert!(pending.is_some());
    assert!(closes_notice(&dialog, &DialogEvent::Cancel));
    assert!(!closes_notice(&dialog, &DialogEvent::None));
}

#[test]
fn a_shown_dialog_keeps_the_notice_waiting() {
    let mut dialog = Some(Dialog::error("Tray failed", "The tray could not start"));
    let pending = notice();

    show_when_free(&mut dialog, &pending);

    assert!(matches!(dialog, Some(Dialog::Error { .. })));
    assert!(!closes_notice(&dialog, &DialogEvent::Cancel));
    dialog = None;
    show_when_free(&mut dialog, &pending);
    assert!(matches!(dialog, Some(Dialog::Notice { .. })));
}

#[test]
fn an_error_shown_over_the_notice_only_puts_it_off() {
    let mut dialog = None;
    let pending = notice();
    show_when_free(&mut dialog, &pending);
    assert!(matches!(dialog, Some(Dialog::Notice { .. })));

    dialog = Some(Dialog::error("Game did not start", "No plugin is assigned"));
    show_when_free(&mut dialog, &pending);
    assert!(matches!(dialog, Some(Dialog::Error { .. })));

    assert!(!closes_notice(&dialog, &DialogEvent::Cancel));
    dialog = None;
    show_when_free(&mut dialog, &pending);
    assert!(matches!(dialog, Some(Dialog::Notice { .. })));
}
