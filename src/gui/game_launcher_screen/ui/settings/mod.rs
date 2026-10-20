mod background;
mod background_preview;
mod behavior;
mod calibration;
pub(in crate::gui::game_launcher_screen) mod calibration_preview;
mod controller;
mod status;

use eframe::egui;
use rust_i18n::t;

use crate::game_launcher_screen::controller::{AdapterRegistry, ControllerDriverDescriptor};
use crate::game_launcher_screen::display::geometry::resolve_profile;
use crate::game_launcher_screen::display::monitor::MonitorDescriptor;
use crate::game_launcher_screen::surface::circle::calibration as circle_calibration;
use crate::game_launcher_screen::ui::layouts::{LauncherLayoutDescriptor, LauncherLayoutRegistry};
use crate::game_launcher_screen::{ControllerConfig, ControllerScreenConfig};
use crate::gui::common::components::{self, DialogContent, ModalDialog};
use background_preview::BackgroundPreview;
use calibration::auto_diameter;
pub(in crate::gui::game_launcher_screen) use status::{LiveButtons, TestStatus};

pub(in crate::gui::game_launcher_screen) const SETTINGS_ID: &str = "game-launcher-screen-settings";

pub(in crate::gui::game_launcher_screen) struct SettingsDraft {
    pub value: ControllerScreenConfig,
    pub monitors: Vec<MonitorDescriptor>,
    pub selected_monitor: Option<usize>,
    pub diameter_px: u32,
    pub preview: bool,
    pub drivers: Vec<&'static ControllerDriverDescriptor>,
    pub layouts: Vec<&'static LauncherLayoutDescriptor>,
    pub test_status: TestStatus,
    pub live_buttons: LiveButtons,
    pub testing: bool,
    pub error: Option<String>,

    pub startup_enabled: bool,

    pub background_preview: BackgroundPreview,
}

pub(in crate::gui::game_launcher_screen) enum SettingsAction {
    None,
    Cancel,
    Save(ControllerScreenConfig),
    Test(ControllerConfig),
    StopTest,
    AutoDetect,
}

impl SettingsDraft {
    pub fn new(value: &ControllerScreenConfig, startup_enabled: bool) -> Result<Self, String> {
        let registry = AdapterRegistry::with_default_adapters();
        registry.validate(&value.controller)?;
        let layouts = LauncherLayoutRegistry::with_builtin_layouts();
        layouts.validate(&value.launcher_layout)?;
        let monitors = crate::game_launcher_screen::display::monitor::enumerate()?;
        let selected_monitor = value
            .monitor_adapter
            .and_then(|wanted| monitors.iter().position(|monitor| monitor.index == wanted))
            .or_else(|| monitors.iter().position(|monitor| monitor.primary))
            .or((!monitors.is_empty()).then_some(0));
        let diameter_px = selected_monitor
            .and_then(|index| {
                let monitor = &monitors[index];
                value.calibration.as_ref().and_then(|calibration| {
                    (calibration.monitor_id == monitor.id.key()).then_some(calibration.diameter_px)
                })
            })
            .or_else(|| {
                selected_monitor
                    .and_then(|index| auto_diameter(&monitors[index], value.orientation).ok())
            })
            .or_else(|| {
                selected_monitor.map(|index| {
                    monitors[index]
                        .bounds
                        .width
                        .min(monitors[index].bounds.height)
                })
            })
            .unwrap_or(1080);
        Ok(Self {
            value: value.clone(),
            monitors,
            selected_monitor,
            diameter_px,
            preview: false,
            drivers: registry.descriptors(),
            layouts: layouts.descriptors(),
            test_status: TestStatus::NotTested,
            live_buttons: LiveButtons::NoReports,
            testing: false,
            error: None,
            startup_enabled,
            background_preview: BackgroundPreview::default(),
        })
    }

    pub fn selected(&self) -> Option<&MonitorDescriptor> {
        self.selected_monitor
            .and_then(|index| self.monitors.get(index))
    }

    pub fn selected_config(&self) -> Result<ControllerScreenConfig, String> {
        let monitor = self
            .selected()
            .ok_or_else(|| t!("controller.validation.select_monitor").to_string())?;
        let mut value = self.value.clone();
        value.monitor_adapter = Some(monitor.index);
        let profile = resolve_profile(value.orientation, monitor.bounds);
        value.calibration = Some(circle_calibration::build(
            monitor.id.key(),
            (monitor.bounds.width, monitor.bounds.height),
            monitor.rotation_degrees,
            profile,
            self.diameter_px,
        )?);
        crate::game_launcher_screen::validate_screen_config(&value)?;
        Ok(value)
    }

    pub fn apply_discovery(
        &mut self,
        found: &crate::game_launcher_screen::controller::DiscoveredController,
    ) {
        self.value.controller = found.config.clone();
        if let Some(driver) = self
            .drivers
            .iter()
            .find(|driver| driver.id == found.config.driver_id)
        {
            self.value.launcher_layout = driver.recommended_layout.into();
        }
        self.test_status = TestStatus::Detected(t!(found.label_key).to_string());
        self.error = None;
    }

    pub fn controller_config(&self) -> Result<ControllerConfig, String> {
        let value = self.value.controller.clone();
        AdapterRegistry::with_default_adapters().validate(&value)?;
        Ok(value)
    }
}

struct SettingsDialog<'a> {
    launcher_dir: &'a std::path::Path,
    draft: &'a mut SettingsDraft,
    action: &'a mut SettingsAction,
}

impl DialogContent for SettingsDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        behavior::show(ui, self.draft);

        ui.separator();
        controller::show(ui, self.draft, self.action);

        ui.separator();
        calibration::show(ui, self.draft);

        ui.add_space(6.0);
        background::show(ui, self.launcher_dir, self.draft);
    }

    fn error(&self) -> Option<&str> {
        self.draft.error.as_deref()
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("common.cancel")).clicked() {
            *self.action = SettingsAction::Cancel;
        }
        if components::action(ui, t!("controller.save")).clicked() {
            match self.draft.selected_config() {
                Ok(config) => *self.action = SettingsAction::Save(config),
                Err(error) => {
                    self.draft.error =
                        Some(t!("controller.error.save", detail => error).to_string())
                }
            }
        }
    }
}

pub(in crate::gui::game_launcher_screen) fn show(
    context: &egui::Context,
    launcher_dir: &std::path::Path,
    draft: &mut SettingsDraft,
) -> SettingsAction {
    let mut action = SettingsAction::None;
    ModalDialog::new(t!("controller.title"), egui::Id::new(SETTINGS_ID)).show(
        context,
        &mut SettingsDialog {
            launcher_dir,
            draft,
            action: &mut action,
        },
    );
    action
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/game_launcher_screen/ui/settings/mod__tests.rs"]
mod tests;
