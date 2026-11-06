use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Seen {
    pub window: isize,
    pub visible: bool,
    pub minimized: bool,

    pub paused: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Action {
    Hide,

    Reveal,

    Show { minimized: bool },
    Nothing,
}

#[derive(Debug, Default)]
pub(super) struct Decisions {
    hidden: BTreeMap<isize, bool>,

    revealed: BTreeSet<isize>,
}

impl Decisions {
    pub fn member(&mut self, seen: &Seen, hiding: bool) -> Action {
        if self.revealed.contains(&seen.window) {
            return Action::Nothing;
        }
        if seen.paused {
            self.revealed.insert(seen.window);
            self.hidden.remove(&seen.window);
            return if seen.visible {
                Action::Nothing
            } else {
                Action::Reveal
            };
        }
        if hiding && seen.visible {
            self.hidden.insert(seen.window, seen.minimized);
            return Action::Hide;
        }
        Action::Nothing
    }

    pub fn stop_hiding(&mut self, exists: impl Fn(isize) -> bool) -> Vec<(isize, bool)> {
        std::mem::take(&mut self.hidden)
            .into_iter()
            .filter(|(window, _)| exists(*window))
            .collect()
    }

    pub fn started_hidden(&self, seen: &Seen) -> Action {
        if seen.visible || self.revealed.contains(&seen.window) {
            Action::Nothing
        } else {
            Action::Show { minimized: false }
        }
    }

    pub fn forget_gone(&mut self, exists: impl Fn(isize) -> bool) {
        self.hidden.retain(|window, _| exists(*window));
        self.revealed.retain(|window| exists(*window));
    }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/services/process/console_watch/decide__tests.rs"]
mod tests;
