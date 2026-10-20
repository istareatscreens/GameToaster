#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoinCondition {
    #[default]
    Normal,

    Jam,

    Disconnected,

    Busy,
}

impl CoinCondition {
    pub fn from_jvs(status: u8) -> Self {
        match status & 0x03 {
            0 => CoinCondition::Normal,
            1 => CoinCondition::Jam,
            2 => CoinCondition::Disconnected,
            _ => CoinCondition::Busy,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoinSlot {
    pub count: u16,

    pub condition: CoinCondition,
}

impl CoinSlot {
    pub fn from_jvs(hi: u8, lo: u8) -> Self {
        let count = (((hi & 0x3F) as u16) << 8) | lo as u16;
        CoinSlot {
            count,
            condition: CoinCondition::from_jvs(hi >> 6),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CoinReading {
    pub slots: Vec<CoinSlot>,
}

pub const COIN_COUNT_MODULUS: u32 = 1 << 14;

pub trait CoinProvider {
    fn name(&self) -> &str;

    fn poll(&mut self) -> Option<CoinReading>;

    fn set_accept(&mut self, accept: bool) -> bool;
}

pub struct CoinMonitor {
    provider: Box<dyn CoinProvider>,
    baseline: Option<Vec<u16>>,
}

impl CoinMonitor {
    pub fn new(provider: Box<dyn CoinProvider>) -> Self {
        Self {
            provider,
            baseline: None,
        }
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub fn poll_new_coins(&mut self) -> u32 {
        let Some(reading) = self.provider.poll() else {
            return 0;
        };
        let counts: Vec<u16> = reading.slots.iter().map(|slot| slot.count).collect();
        let new_coins = match &self.baseline {
            Some(previous) if previous.len() == counts.len() => previous
                .iter()
                .zip(counts.iter())
                .map(|(&was, &now)| (now as u32).wrapping_sub(was as u32) % COIN_COUNT_MODULUS)
                .sum(),
            _ => 0,
        };
        self.baseline = Some(counts);
        new_coins
    }

    pub fn set_accept(&mut self, accept: bool) -> bool {
        self.provider.set_accept(accept)
    }
}

pub struct SimulatedCoinProvider {
    readings: std::collections::VecDeque<Option<CoinReading>>,
    accept: bool,
    can_gate: bool,
}

impl SimulatedCoinProvider {
    pub fn new(readings: Vec<Option<CoinReading>>) -> Self {
        Self {
            readings: readings.into_iter().collect(),
            accept: true,
            can_gate: true,
        }
    }

    pub fn without_gating(mut self) -> Self {
        self.can_gate = false;
        self
    }

    pub fn accepting(&self) -> bool {
        self.accept
    }
}

impl CoinProvider for SimulatedCoinProvider {
    fn name(&self) -> &str {
        "sim"
    }
    fn poll(&mut self) -> Option<CoinReading> {
        match self.readings.len() {
            0 => None,
            1 => self.readings.front().cloned().flatten(),
            _ => self.readings.pop_front().flatten(),
        }
    }
    fn set_accept(&mut self, accept: bool) -> bool {
        if self.can_gate {
            self.accept = accept;
        }
        self.can_gate
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/coin__tests.rs"]
mod tests;
