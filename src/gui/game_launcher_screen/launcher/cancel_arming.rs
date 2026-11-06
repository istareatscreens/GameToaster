use std::time::Duration;

pub(crate) const ARM_AFTER: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct CancelArming {
    armed: bool,
}

impl CancelArming {
    pub fn begin(&mut self) {
        self.armed = false;
    }

    pub fn released(&mut self, since_launch: Duration) {
        if since_launch >= ARM_AFTER {
            self.armed = true;
        }
    }

    pub fn may_cancel(self) -> bool {
        self.armed
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/cancel_arming__tests.rs"]
mod tests;
