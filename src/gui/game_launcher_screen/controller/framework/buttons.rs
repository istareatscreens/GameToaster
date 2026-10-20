#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum Button {
    Coin = 0,
    Ba1 = 1,
    Ba2 = 2,
    Ba3 = 3,
    Ba4 = 4,
    Ba5 = 5,
    Ba6 = 6,
    Ba7 = 7,
    Ba8 = 8,
    Select = 9,
    Up = 10,
    Down = 11,
}

impl Button {
    pub const ALL: [Self; 12] = [
        Self::Coin,
        Self::Ba1,
        Self::Ba2,
        Self::Ba3,
        Self::Ba4,
        Self::Ba5,
        Self::Ba6,
        Self::Ba7,
        Self::Ba8,
        Self::Select,
        Self::Up,
        Self::Down,
    ];

    pub const fn mask(self) -> u16 {
        1 << self as u8
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Coin => "COIN",
            Self::Ba1 => "BA1",
            Self::Ba2 => "BA2",
            Self::Ba3 => "BA3",
            Self::Ba4 => "BA4",
            Self::Ba5 => "BA5",
            Self::Ba6 => "BA6",
            Self::Ba7 => "BA7",
            Self::Ba8 => "BA8",
            Self::Select => "SELECT",
            Self::Up => "UP",
            Self::Down => "DOWN",
        }
    }
}

const ALL_BUTTONS: u16 = {
    let mut mask = 0;
    let mut index = 0;
    while index < Button::ALL.len() {
        mask |= Button::ALL[index].mask();
        index += 1;
    }
    mask
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ButtonWord(u16);

impl ButtonWord {
    pub const EMPTY: Self = Self(0);

    pub const fn from_bits(bits: u16) -> Self {
        Self(bits & ALL_BUTTONS)
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, button: Button) -> bool {
        self.0 & button.mask() != 0
    }

    pub fn set(&mut self, button: Button) {
        self.0 |= button.mask();
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/controller/framework/buttons__tests.rs"]
mod tests;
