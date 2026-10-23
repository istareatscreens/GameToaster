use std::path::PathBuf;

use eframe::egui;

use crate::launcher_gui::config::Notice;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::LauncherConfig;

use super::LauncherApp;

pub fn run(
    launcher_dir: PathBuf,
    initial_config: LauncherConfig,
    config_notice: Option<Notice>,
    log: LauncherLog,
    start_in_tray: bool,
) -> Result<(), String> {
    {
        let panic_log = log.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown".into());
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic payload>".into());

            panic_log.crash("panic", &[("location", location), ("message", message)]);
            previous(info);
        }));
    }

    crate::launcher_gui::logging::BackendLogSink::install(&log);

    let main_visible_on_start =
        main_window_starts_visible(start_in_tray, &initial_config, config_notice.is_some());
    let (rgba, width, height) = crate::launcher_gui::assets::icon_rgba(&launcher_dir, 256)?;
    let viewport_icon = egui::IconData {
        rgba,
        width,
        height,
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("GameToaster")
            .with_app_id("gametoaster-launcher")
            .with_inner_size([1080.0, 800.0])
            .with_min_inner_size([1080.0, 800.0])
            .with_max_inner_size([1080.0, 800.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_visible(main_visible_on_start)
            .with_icon(viewport_icon),
        ..Default::default()
    };
    eframe::run_native(
        "GameToaster",
        options,
        Box::new(move |creation| {
            Ok(Box::new(LauncherApp::new(
                creation,
                launcher_dir,
                initial_config,
                config_notice,
                log,
                start_in_tray,
                main_visible_on_start,
            )))
        }),
    )
    .map_err(|error| crate::launcher_gui::startup_failure::gui_failure_message(&error.to_string()))
}

fn main_window_starts_visible(
    start_in_tray: bool,
    config: &LauncherConfig,
    notice_pending: bool,
) -> bool {
    notice_pending || (!start_in_tray && !config.settings.controller_screen.starts_main_hidden())
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/run__tests.rs"]
mod tests;
