use eframe::egui;
use rust_i18n::t;

use crate::gui::common::components::{self, DialogContent};

use super::DialogEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tone {
    Error,
    Notice,
}

pub(super) struct MessageDialog<'a> {
    title: &'a str,
    message: &'a str,
    tone: Tone,
    event: &'a mut DialogEvent,
}

impl<'a> MessageDialog<'a> {
    pub(super) fn new(
        title: &'a str,
        message: &'a str,
        tone: Tone,
        event: &'a mut DialogEvent,
    ) -> Self {
        Self {
            title,
            message,
            tone,
            event,
        }
    }
}

impl DialogContent for MessageDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        match self.tone {
            Tone::Error => components::error_text(ui, self.message),
            Tone::Notice => components::text::copyable_text(ui, self.message, None),
        };
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("common.ok")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
        let payload = format!("{}\n{}", self.title, self.message);
        components::copy_button(ui, t!("common.copy"), &payload);
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/message__tests.rs"]
mod tests;
