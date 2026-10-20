use super::buttons::ButtonWord;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen::controller) struct ButtonEdges {
    pub rising: ButtonWord,
    pub falling: ButtonWord,
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::gui::game_launcher_screen::controller) struct EdgeTracker {
    previous: ButtonWord,
    suppressed_until_release: ButtonWord,
}

impl EdgeTracker {
    pub fn establish_baseline(&mut self, snapshot: ButtonWord) {
        self.previous = snapshot;
        self.suppressed_until_release = snapshot;
    }

    pub fn update(&mut self, snapshot: ButtonWord) -> ButtonEdges {
        let changed = self.previous.bits() ^ snapshot.bits();
        let rising = changed & snapshot.bits() & !self.suppressed_until_release.bits();
        let falling = changed & self.previous.bits();
        self.suppressed_until_release =
            ButtonWord::from_bits(self.suppressed_until_release.bits() & snapshot.bits());
        self.previous = snapshot;
        ButtonEdges {
            rising: ButtonWord::from_bits(rising),
            falling: ButtonWord::from_bits(falling),
        }
    }

    pub fn disconnect(&mut self) -> ButtonEdges {
        let falling = self.previous;
        self.previous = ButtonWord::EMPTY;
        self.suppressed_until_release = ButtonWord::EMPTY;
        ButtonEdges {
            rising: ButtonWord::EMPTY,
            falling,
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/edges__tests.rs"]
mod tests;
