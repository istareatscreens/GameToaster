mod textures;
mod washing_machine;

use std::collections::BTreeMap;
use std::path::Path;

use eframe::egui;

use crate::gui::common::message::{message, Message};
use crate::launcher_gui::model::LauncherConfig;

use super::super::display::geometry::PixelRect;
use super::super::domain::circular_game_list::CircularGameList;
use super::super::domain::command::LauncherCommand;
use super::super::surface::SurfaceShape;

use washing_machine::WashingMachineLayout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct LauncherLayoutDescriptor {
    pub id: &'static str,
    pub label_key: &'static str,

    pub surface: SurfaceShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) enum LauncherFrameMode<'a> {
    Menu { credit_readout: Option<&'a str> },
    Launching { status: &'a str },
}

pub(in crate::gui::game_launcher_screen) struct LauncherFrame<'a> {
    pub launcher_dir: &'a Path,
    pub config: &'a LauncherConfig,

    pub background: Option<&'a Path>,
    pub games: &'a CircularGameList,

    pub surface_px: PixelRect,
    pub mode: LauncherFrameMode<'a>,
}

pub(in crate::gui::game_launcher_screen) trait LauncherScreenLayout:
    Send
{
    fn descriptor(&self) -> &'static LauncherLayoutDescriptor;
    fn activate(&mut self) {}
    fn library_changed(&mut self, config: &LauncherConfig);
    fn deactivate(&mut self);
    fn render(&mut self, ui: &mut egui::Ui, frame: &LauncherFrame<'_>) -> Option<LauncherCommand>;
    #[cfg(test)]
    fn resource_count(&self) -> usize;
}

#[derive(Default)]
pub(in crate::gui::game_launcher_screen) struct LauncherLayoutRegistry {
    layouts: BTreeMap<&'static str, Box<dyn LauncherScreenLayout>>,
    active: Option<&'static str>,
}

impl LauncherLayoutRegistry {
    pub fn with_builtin_layouts() -> Self {
        let mut registry = Self::default();
        registry
            .register(Box::new(WashingMachineLayout::default()))
            .expect("built-in launcher layouts must have valid unique IDs");
        registry
    }

    pub fn register(&mut self, layout: Box<dyn LauncherScreenLayout>) -> Result<(), Message> {
        let descriptor = layout.descriptor();
        validate_id(descriptor.id)?;
        if self.layouts.contains_key(descriptor.id) {
            return Err(message!(
                "controller.layout.error.already_registered",
                id => descriptor.id
            ));
        }
        self.layouts.insert(descriptor.id, layout);
        Ok(())
    }

    pub fn surface_shape(&self, layout_id: &str) -> SurfaceShape {
        self.layouts
            .get(layout_id)
            .map(|layout| layout.descriptor().surface)
            .unwrap_or_default()
    }

    pub fn descriptors(&self) -> Vec<&'static LauncherLayoutDescriptor> {
        self.layouts
            .values()
            .map(|layout| layout.descriptor())
            .collect()
    }

    pub fn validate(&self, layout_id: &str) -> Result<(), Message> {
        validate_id(layout_id)?;
        if self.layouts.contains_key(layout_id) {
            Ok(())
        } else {
            Err(message!("controller.layout.error.not_registered", id => layout_id))
        }
    }

    pub fn activate(&mut self, layout_id: &str) -> Result<(), Message> {
        self.validate(layout_id)?;
        if self.active == Some(layout_id) {
            return Ok(());
        }
        let key = self
            .layouts
            .get_key_value(layout_id)
            .map(|(key, _)| *key)
            .ok_or_else(|| message!("controller.layout.error.not_registered", id => layout_id))?;
        self.deactivate();
        let layout = self
            .layouts
            .get_mut(key)
            .ok_or_else(|| message!("controller.layout.error.disappeared", id => layout_id))?;
        layout.activate();
        self.active = Some(key);
        Ok(())
    }

    pub fn active_mut(
        &mut self,
        layout_id: &str,
    ) -> Result<&mut (dyn LauncherScreenLayout + '_), Message> {
        if self.active != Some(layout_id) {
            return Err(message!("controller.layout.error.not_active", id => layout_id));
        }
        match self.layouts.get_mut(layout_id) {
            Some(layout) => Ok(layout.as_mut()),
            None => Err(message!("controller.layout.error.not_registered", id => layout_id)),
        }
    }

    pub fn library_changed(&mut self, config: &LauncherConfig) {
        if let Some(active) = self.active {
            if let Some(layout) = self.layouts.get_mut(active) {
                layout.library_changed(config);
            }
        }
    }

    pub fn deactivate(&mut self) {
        self.release_resources();
        self.active = None;
    }

    pub fn release_resources(&mut self) {
        if let Some(active) = self.active {
            if let Some(layout) = self.layouts.get_mut(active) {
                layout.deactivate();
            }
        }
    }

    #[cfg(test)]
    pub fn resource_count(&self) -> usize {
        self.layouts
            .values()
            .map(|layout| layout.resource_count())
            .sum()
    }
}

fn validate_id(id: &str) -> Result<(), Message> {
    if !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(())
    } else {
        Err(message!("controller.layout.error.invalid_id"))
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/ui/layouts/mod__tests.rs"]
mod tests;
