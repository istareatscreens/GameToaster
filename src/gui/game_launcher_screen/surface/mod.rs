pub(super) mod circle;

use super::display::geometry::PixelRect;
use super::display::monitor::MonitorDescriptor;
use crate::game_launcher_screen::ControllerScreenConfig;
use crate::gui::common::message::Message;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum SurfaceShape {
    #[default]
    Circle,
}

pub(in crate::gui::game_launcher_screen) fn resolve(
    shape: SurfaceShape,
    monitor: &MonitorDescriptor,
    config: &ControllerScreenConfig,
) -> Result<PixelRect, Message> {
    match shape {
        SurfaceShape::Circle => circle::resolve(monitor, config),
    }
}
