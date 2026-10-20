use std::sync::Arc;
use std::time::Duration;

use crate::game_launcher_screen::ControllerConfig;

use super::audit::ResourceAudit;
use super::buttons::ButtonWord;
use super::descriptor::{ControllerDriverDescriptor, DiscoveredController};
use super::events::EventSink;

pub(in crate::gui::game_launcher_screen::controller) trait ButtonDecoder:
    Send + Sync
{
    fn decode(&self, report: &[u8]) -> Result<ButtonWord, String>;
}

pub(in crate::gui::game_launcher_screen::controller) trait ControllerSession:
    Send
{
    fn disconnect(&mut self, timeout: Duration) -> Result<(), String>;
}

pub(in crate::gui::game_launcher_screen::controller) trait ControllerAdapter:
    Send + Sync
{
    fn descriptor(&self) -> &'static ControllerDriverDescriptor;

    fn validate(&self, config: &ControllerConfig) -> Result<(), String> {
        self.descriptor().validate(config)
    }

    fn discover(&self) -> Result<Vec<DiscoveredController>, String> {
        Ok(Vec::new())
    }

    fn connect(
        &self,
        config: &ControllerConfig,
        events: EventSink,
        audit: Arc<ResourceAudit>,
    ) -> Result<Box<dyn ControllerSession>, String>;
}
