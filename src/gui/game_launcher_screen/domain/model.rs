use rust_i18n::t;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LauncherXAction {
    MinimizeToTray,
    ExitLauncher,
}

impl fmt::Display for LauncherXAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MinimizeToTray => "minimize-to-tray",
            Self::ExitLauncher => "exit-launcher",
        })
    }
}

impl FromStr for LauncherXAction {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "minimize-to-tray" | "minimize" => Ok(Self::MinimizeToTray),
            "exit-launcher" | "exit" | "close" => Ok(Self::ExitLauncher),
            _ => Err(format!("unknown launcher X action '{value}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OrientationProfile {
    Auto,
    CabinetPortrait,
    ProductionLandscape,
}

impl fmt::Display for OrientationProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::CabinetPortrait => "cabinet-portrait",
            Self::ProductionLandscape => "production-landscape",
        })
    }
}

impl FromStr for OrientationProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "cabinet-portrait" | "portrait" => Ok(Self::CabinetPortrait),
            "production-landscape" | "landscape" => Ok(Self::ProductionLandscape),
            _ => Err(format!("unknown controller display profile '{value}'")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MonitorCalibration {
    pub monitor_id: String,
    pub mode_width_px: u32,
    pub mode_height_px: u32,
    pub rotation_degrees: u16,
    pub profile: OrientationProfile,
    pub diameter_px: u32,
}

const DEFAULT_CONTROLLER_DRIVER: &str = "adx-io4";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ControllerConfig {
    pub driver_id: String,
    pub options: BTreeMap<String, String>,
}

impl ControllerConfig {
    pub fn new<K, V>(
        driver_id: impl Into<String>,
        options: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            driver_id: driver_id.into(),
            options: options
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    pub fn validate_shape(&self) -> Result<(), String> {
        if self.driver_id.is_empty()
            || !self
                .driver_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(
                "controller driver ID must use lowercase letters, digits, and hyphens".into(),
            );
        }
        for (key, value) in &self.options {
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            {
                return Err(format!(
                    "controller option key '{key}' must use lowercase letters, digits, and underscores"
                ));
            }
            if value.contains(['\r', '\n']) {
                return Err(format!("controller option '{key}' must fit on one line"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ControllerScreenConfig {
    pub enabled: bool,
    pub launch_on_start: bool,
    pub reopen_after_game_close: bool,

    pub reopen_on_monitor_return: bool,
    pub controller_only_start: bool,

    pub open_on_os_startup: bool,

    pub ba1_enabled: bool,

    pub ba2_enabled: bool,

    pub keep_controller_alive_when_minimized: bool,
    pub launcher_x_action: LauncherXAction,

    pub monitor_adapter: Option<u32>,

    pub background_image: Option<std::path::PathBuf>,

    pub launcher_layout: String,
    pub orientation: OrientationProfile,
    pub calibration: Option<MonitorCalibration>,
    pub controller: ControllerConfig,
}

impl Default for ControllerScreenConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            launch_on_start: false,
            reopen_after_game_close: true,
            reopen_on_monitor_return: false,
            controller_only_start: false,
            open_on_os_startup: false,
            ba1_enabled: true,
            ba2_enabled: true,
            keep_controller_alive_when_minimized: false,
            launcher_x_action: LauncherXAction::MinimizeToTray,
            monitor_adapter: None,
            background_image: None,
            launcher_layout: "washing-machine".into(),
            orientation: OrientationProfile::Auto,
            calibration: None,
            controller: crate::game_launcher_screen::default_controller_config(
                DEFAULT_CONTROLLER_DRIVER,
            )
            .expect("the default controller driver is built in"),
        }
    }
}

impl ControllerScreenConfig {
    pub fn starts_main_hidden(&self) -> bool {
        self.enabled && self.launch_on_start && self.controller_only_start
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.controller_only_start && !self.launch_on_start {
            return Err(t!("controller.validation.controller_only_start").to_string());
        }
        self.controller.validate_shape()?;
        if self.launcher_layout.is_empty()
            || !self
                .launcher_layout
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(t!("controller.layout.error.invalid_id").to_string());
        }
        if let Some(calibration) = &self.calibration {
            if calibration.monitor_id.trim().is_empty() {
                return Err("controller display calibration has an empty monitor identity".into());
            }
            if calibration.diameter_px == 0 {
                return Err("controller display calibration diameter must be positive".into());
            }
            if calibration.diameter_px > calibration.mode_width_px
                || calibration.diameter_px > calibration.mode_height_px
            {
                return Err("the calibration circle does not fit the calibrated mode".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/domain/model__tests.rs"]
mod tests;
