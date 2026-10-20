use eframe::egui;
use rust_i18n::t;

use super::SettingsDraft;
use crate::game_launcher_screen::display::geometry::resolve_profile;
use crate::game_launcher_screen::display::monitor::MonitorDescriptor;
use crate::game_launcher_screen::surface::circle::calibration;
use crate::game_launcher_screen::OrientationProfile;
use crate::gui::common::components::Combo;

pub(super) fn show(ui: &mut egui::Ui, draft: &mut SettingsDraft) {
    ui.heading(t!("controller.calib.heading").to_string());
    let previous_monitor = draft.selected_monitor;
    Combo::labelled(t!("controller.calib.monitor"), &mut draft.selected_monitor)
        .choices(
            draft
                .monitors
                .iter()
                .enumerate()
                .map(|(index, monitor)| (Some(index), monitor_label(monitor))),
        )
        .placeholder(t!("controller.calib.no_monitor"))
        .show(ui);
    let orientation = draft.value.orientation;
    if previous_monitor != draft.selected_monitor {
        if let Some(monitor) = draft.selected().cloned() {
            draft.diameter_px = auto_diameter(&monitor, orientation)
                .unwrap_or_else(|_| monitor.bounds.width.min(monitor.bounds.height));
        }
    }
    Combo::labelled(t!("controller.calib.profile"), &mut draft.value.orientation)
        .choices(
            [
                OrientationProfile::Auto,
                OrientationProfile::CabinetPortrait,
                OrientationProfile::ProductionLandscape,
            ]
            .map(|profile| (profile, profile_label(profile))),
        )
        .show(ui);
    if let Some(monitor) = draft.selected().cloned() {
        let maximum = monitor.bounds.width.min(monitor.bounds.height).max(1);
        if draft.diameter_px > maximum {
            draft.diameter_px = maximum;
        }
        ui.label(
            t!(
                "controller.calib.mode_line",
                w => monitor.bounds.width,
                h => monitor.bounds.height,
                rot => monitor.rotation_degrees,
                dpix => monitor.dpi_x,
                dpiy => monitor.dpi_y,
            )
            .to_string(),
        );
        ui.add(
            egui::Slider::new(&mut draft.diameter_px, 1..=maximum)
                .text(t!("controller.calib.diameter").to_string()),
        );
        ui.horizontal(|ui| {
            if crate::gui::common::components::action(
                ui,
                t!("controller.calib.auto_detect").to_string(),
            )
            .clicked()
            {
                match auto_diameter(&monitor, orientation) {
                    Ok(value) => draft.diameter_px = value.min(maximum),
                    Err(error) => {
                        draft.error = Some(
                            t!("controller.error.detect_diameter", detail => error).to_string(),
                        )
                    }
                }
            }
            if crate::gui::common::components::action(
                ui,
                if draft.preview {
                    t!("controller.calib.preview_close").to_string()
                } else {
                    t!("controller.calib.preview_open").to_string()
                },
            )
            .clicked()
            {
                draft.preview = !draft.preview;
            }
        });
    }
}

pub(super) fn auto_diameter(
    monitor: &MonitorDescriptor,
    orientation: OrientationProfile,
) -> Result<u32, String> {
    let profile = resolve_profile(orientation, monitor.bounds);
    calibration::auto_diameter(
        profile,
        monitor.bounds.width,
        monitor.bounds.height,
        monitor.physical_width_mm,
        monitor.physical_height_mm,
    )
}

fn monitor_label(monitor: &MonitorDescriptor) -> String {
    let primary = if monitor.primary {
        format!(" ({})", t!("controller.calib.primary"))
    } else {
        String::new()
    };
    format!(
        "{} - {}x{}{}",
        monitor.friendly_name, monitor.bounds.width, monitor.bounds.height, primary,
    )
}

fn profile_label(profile: OrientationProfile) -> String {
    match profile {
        OrientationProfile::Auto => t!("controller.calib.auto"),
        OrientationProfile::CabinetPortrait => t!("controller.calib.bottom"),
        OrientationProfile::ProductionLandscape => t!("controller.calib.centered"),
    }
    .to_string()
}
