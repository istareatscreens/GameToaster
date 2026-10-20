#![windows_subsystem = "windows"]

mod gui;
mod plugin;
mod support;
pub(crate) use gui::game_launcher_screen;
pub(crate) use gui::main_window as launcher_gui;

rust_i18n::i18n!("locales", fallback = "en");

use support::win::wide;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MESSAGEBOX_STYLE};

use launcher_gui::logging::LauncherLog;

fn main() -> std::process::ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            show_fatal_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: &[std::ffi::OsString]) -> Result<(), String> {
    let windows_language = launcher_gui::i18n::detect_os_locale();
    launcher_gui::i18n::apply(windows_language);
    set_per_monitor_v2_dpi_awareness();
    let install_kind = launcher_gui::platform::package::InstallKind::current()?;
    let launcher_dir = launcher_gui::platform::storage::state_root(install_kind)?;
    let log = LauncherLog::open(&launcher_dir)?;

    launcher_gui::i18n::apply("en");
    let opened = launcher_gui::config::open(&launcher_dir);
    launcher_gui::i18n::apply(windows_language);
    let opened = opened.inspect_err(|error| {
        log.crash(
            "config-load",
            &[("result", "rejected".into()), ("error", error.clone())],
        );
    })?;
    let config = opened.config;
    log.set_enabled(config.settings.diagnostics_logging);

    let start_in_tray = launcher_gui::platform::startup::launched_at_sign_in(args);
    launcher_gui::run(launcher_dir, config, opened.notice, log, start_in_tray)
}

fn set_per_monitor_v2_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

fn show_fatal_error(error: &str) {
    let title = wide(&*rust_i18n::t!("startup.error_title"));
    let message = wide(error);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MESSAGEBOX_STYLE(MB_OK.0 | MB_ICONERROR.0),
        );
    }
}
