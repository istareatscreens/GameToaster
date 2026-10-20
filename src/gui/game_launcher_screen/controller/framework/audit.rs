use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub(in crate::gui::game_launcher_screen) struct ResourceAudit {
    open_handles: AtomicUsize,
    live_readers: AtomicUsize,
}

impl ResourceAudit {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(in crate::gui::game_launcher_screen::controller) fn handle_opened(&self) {
        self.open_handles.fetch_add(1, Ordering::AcqRel);
    }

    pub(in crate::gui::game_launcher_screen::controller) fn handle_closed(&self) {
        self.open_handles.fetch_sub(1, Ordering::AcqRel);
    }

    pub(in crate::gui::game_launcher_screen::controller) fn reader_started(&self) {
        self.live_readers.fetch_add(1, Ordering::AcqRel);
    }

    pub(in crate::gui::game_launcher_screen::controller) fn reader_stopped(&self) {
        self.live_readers.fetch_sub(1, Ordering::AcqRel);
    }

    pub fn counts(&self) -> (usize, usize) {
        (
            self.open_handles.load(Ordering::Acquire),
            self.live_readers.load(Ordering::Acquire),
        )
    }

    pub fn proof(&self) -> ReleaseProof {
        let (open_handles, live_readers) = self.counts();
        ReleaseProof {
            open_handles,
            live_readers,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct ReleaseProof {
    pub open_handles: usize,
    pub live_readers: usize,
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/audit__tests.rs"]
mod tests;
