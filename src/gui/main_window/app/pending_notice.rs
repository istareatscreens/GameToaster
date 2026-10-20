use crate::launcher_gui::config::Notice;
use crate::launcher_gui::dialogs::{self, Dialog, DialogEvent};

pub(super) fn show_when_free(dialog: &mut Option<Dialog>, notice: &Option<Notice>) {
    if dialog.is_none() {
        *dialog = notice.as_ref().map(dialogs::config_notice);
    }
}

pub(super) fn closes_notice(dialog: &Option<Dialog>, event: &DialogEvent) -> bool {
    matches!(dialog, Some(Dialog::Notice { .. })) && matches!(event, DialogEvent::Cancel)
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/pending_notice__tests.rs"]
mod tests;
