use std::path::PathBuf;

use super::super::harness;
use super::*;
use crate::gui::common::harness::{locale, painted_texts};

const KEPT: &str = r"C:\State\GameToaster.ini.schema-2.20261018T093000.123Z.bak";

fn texts(dialog: &Dialog) -> (&str, &str) {
    match dialog {
        Dialog::Notice { title, message } => (title, message),
        _ => panic!("not a notice"),
    }
}

fn upgraded(plugin_needed: &[&str]) -> Notice {
    Notice::Upgraded {
        kept: PathBuf::from(KEPT),
        plugin_needed: plugin_needed
            .iter()
            .map(|label| label.to_string())
            .collect(),
    }
}

fn reset(reason: ResetReason) -> Notice {
    Notice::Reset {
        reason,
        kept: PathBuf::from(KEPT),
    }
}

#[test]
fn an_upgrade_says_where_the_file_as_it_was_is_kept() {
    let _locale = locale("en");
    let dialog = config_notice(&upgraded(&[]));
    let (title, message) = texts(&dialog);
    assert_eq!(title, "Settings converted");
    assert_eq!(
        message,
        format!(
            "GameToaster converted your settings for this version. The file as it was is kept at:\n{KEPT}"
        )
    );
}

#[test]
fn an_upgrade_lists_the_games_that_need_a_plugin_one_per_line() {
    let _locale = locale("en");
    let dialog = config_notice(&upgraded(&["First Game", "日本語 ゲーム"]));
    let (_, message) = texts(&dialog);
    assert!(
        message.ends_with(
            "\n\nThe previous launcher set up these games itself each time it started them. Assign each its plugin before you launch it:\nFirst Game\n日本語 ゲーム"
        ),
        "{message}"
    );
}

#[test]
fn a_reset_says_where_the_file_is_kept_and_why() {
    let _locale = locale("en");
    for (reason, said) in [
        (
            ResetReason::UnknownVersion,
            "The file does not say which version of GameToaster wrote it.".to_string(),
        ),
        (
            ResetReason::Newer(4),
            "The file was written by a newer version of GameToaster (settings format 4)."
                .to_string(),
        ),
        (
            ResetReason::Unreadable("stream did not contain valid UTF-8".into()),
            "The file could not be read: stream did not contain valid UTF-8".to_string(),
        ),
        (
            ResetReason::NotUpgradable("unknown controller adapter 'keyboard'".into()),
            "The file could not be converted: unknown controller adapter 'keyboard'".to_string(),
        ),
    ] {
        let dialog = config_notice(&reset(reason));
        let (title, message) = texts(&dialog);
        assert_eq!(title, "Settings reset");
        assert_eq!(
            message,
            format!(
                "GameToaster could not use your settings and started with the default settings. Your file is kept at:\n{KEPT}\n\n{said}"
            )
        );
    }
}

#[test]
fn the_notice_renders_its_title_and_text_in_every_language() {
    for code in ["en", "ja", "zh-Hans", "zh-Hant"] {
        let _locale = locale(code);
        for notice in [upgraded(&["First Game"]), reset(ResetReason::Newer(4))] {
            let dialog = config_notice(&notice);
            let (title, message) = texts(&dialog);
            let (title, message) = (title.to_string(), message.to_string());
            let shown = harness::show(dialog);
            let painted: Vec<String> = painted_texts(&shown.output)
                .into_iter()
                .map(|(text, _)| text)
                .collect();
            assert!(painted.contains(&title), "{code}: {title:?} in {painted:?}");
            assert!(
                painted.contains(&message),
                "{code}: {message:?} in {painted:?}"
            );
            assert_eq!(shown.actions().len(), 2, "{code}");
        }
    }
}
