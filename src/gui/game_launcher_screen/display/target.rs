use super::monitor::MonitorDescriptor;
use crate::game_launcher_screen::ControllerScreenConfig;
use crate::gui::common::message::{message, Message};

pub(in crate::gui::game_launcher_screen) fn resolve(
    config: &ControllerScreenConfig,
) -> Result<MonitorDescriptor, Message> {
    let monitors = super::monitor::enumerate()?;
    if config.reopen_on_monitor_return
        && has_configured_display(config)
        && !configured_target_present(&monitors, config)
    {
        return Err(message!("controller.display.configured_inactive"));
    }

    let wanted_id = config
        .calibration
        .as_ref()
        .map(|calibration| calibration.monitor_id.as_str());
    let monitor = super::monitor::select(&monitors, wanted_id, config.monitor_adapter)
        .ok_or_else(|| message!("controller.display.no_active_monitor"))?;
    Ok(monitor.clone())
}

pub(crate) fn configured_display_available(config: &ControllerScreenConfig) -> bool {
    let Ok(monitors) = super::monitor::enumerate() else {
        return false;
    };
    configured_target_present(&monitors, config)
}

pub(crate) fn describe_monitors(saved_id: Option<&str>) -> Result<Vec<String>, String> {
    let monitors = super::monitor::enumerate()?;
    Ok(monitors
        .iter()
        .map(|monitor| {
            let key = monitor.id.key();
            let matches = saved_id
                .is_some_and(|wanted| super::monitor::same_physical_monitor(&key, wanted));
            format!(
                "index={} name={} path={} bounds={}x{}@{},{} rotation={} dpi={}x{} native_pixels={} primary={} friendly={} matches_saved={} key={}",
                monitor.index,
                monitor.id.device_name,
                monitor.id.device_path,
                monitor.bounds.width,
                monitor.bounds.height,
                monitor.bounds.x,
                monitor.bounds.y,
                monitor.rotation_degrees,
                monitor.dpi_x,
                monitor.dpi_y,
                monitor.native_pixels,
                monitor.primary,
                monitor.friendly_name,
                matches,
                key
            )
        })
        .collect())
}

fn has_configured_display(config: &ControllerScreenConfig) -> bool {
    config
        .calibration
        .as_ref()
        .is_some_and(|calibration| !calibration.monitor_id.trim().is_empty())
        || config.monitor_adapter.is_some()
}

fn configured_target_present(
    monitors: &[MonitorDescriptor],
    config: &ControllerScreenConfig,
) -> bool {
    if let Some(wanted_id) = config
        .calibration
        .as_ref()
        .map(|calibration| calibration.monitor_id.trim())
        .filter(|id| !id.is_empty())
    {
        return monitors
            .iter()
            .any(|monitor| super::monitor::same_physical_monitor(&monitor.id.key(), wanted_id));
    }
    if let Some(adapter) = config.monitor_adapter {
        return monitors.iter().any(|monitor| monitor.index == adapter);
    }
    !monitors.is_empty()
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/display/target__tests.rs"]
mod tests;
