use super::super::display::geometry::PixelRect;
use super::super::display::monitor::MonitorDescriptor;
use super::super::surface::SurfaceShape;
use super::super::{display, surface, ControllerScreenConfig};
use crate::gui::common::message::Message;

#[derive(Clone)]
pub(super) struct ResolvedDisplay {
    pub(super) monitor: MonitorDescriptor,

    pub(super) surface: PixelRect,
}

pub(super) fn resolve_display(
    config: &ControllerScreenConfig,
    shape: SurfaceShape,
) -> Result<ResolvedDisplay, Message> {
    let monitor = display::target::resolve(config)?;
    let surface = surface::resolve(shape, &monitor, config)?;
    Ok(ResolvedDisplay { monitor, surface })
}
