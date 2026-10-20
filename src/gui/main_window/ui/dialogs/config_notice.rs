use rust_i18n::t;

use crate::launcher_gui::config::{Notice, ResetReason};

use super::Dialog;

pub fn config_notice(notice: &Notice) -> Dialog {
    match notice {
        Notice::Upgraded {
            kept,
            plugin_needed,
        } => {
            let mut body =
                t!("config_notice.upgraded_body", path => kept.display().to_string()).to_string();
            if !plugin_needed.is_empty() {
                body.push_str("\n\n");
                body.push_str(&t!(
                    "config_notice.plugin_needed",
                    games => plugin_needed.join("\n")
                ));
            }
            Dialog::notice(t!("config_notice.upgraded_title"), body)
        }
        Notice::Reset { reason, kept } => {
            let reason = match reason {
                ResetReason::UnknownVersion => t!("config_notice.unknown_version"),
                ResetReason::Newer(version) => t!("config_notice.newer", found => version),
                ResetReason::Unreadable(detail) => {
                    t!("config_notice.unreadable", detail => detail)
                }
                ResetReason::NotUpgradable(detail) => {
                    t!("config_notice.not_upgradable", detail => detail)
                }
            };
            Dialog::notice(
                t!("config_notice.reset_title"),
                t!(
                    "config_notice.reset_body",
                    path => kept.display().to_string(),
                    reason => reason
                ),
            )
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/config_notice__tests.rs"]
mod tests;
