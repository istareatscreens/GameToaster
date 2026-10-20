use std::cell::Cell;
use std::sync::mpsc::{SyncSender, TrySendError};

use eframe::egui;

use super::buttons::ButtonWord;

#[derive(Clone, Debug)]
pub(in crate::gui::game_launcher_screen::controller) enum SessionEvent {
    Connected,
    Snapshot(ButtonWord),
    Error(String),
    Disconnected,
}

pub(in crate::gui::game_launcher_screen::controller) struct EventSink {
    inner: SyncSender<SessionEvent>,

    ctx: Option<egui::Context>,

    last: Cell<Option<ButtonWord>>,
}

impl EventSink {
    pub fn new(inner: SyncSender<SessionEvent>, ctx: Option<egui::Context>) -> Self {
        Self {
            inner,
            ctx,
            last: Cell::new(None),
        }
    }

    pub fn try_send(&self, event: SessionEvent) -> Result<(), TrySendError<SessionEvent>> {
        let wake = match &event {
            SessionEvent::Snapshot(word) => {
                if self.last.get() == Some(*word) {
                    return Ok(());
                }
                self.last.set(Some(*word));
                true
            }

            SessionEvent::Connected | SessionEvent::Error(_) | SessionEvent::Disconnected => true,
        };
        let result = self.inner.try_send(event);
        if result.is_ok() && wake {
            if let Some(ctx) = &self.ctx {
                ctx.request_repaint();
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/events__tests.rs"]
mod tests;
