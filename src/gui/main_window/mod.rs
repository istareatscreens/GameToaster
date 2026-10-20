pub mod app;
pub mod domain;
pub mod platform;
pub mod services;
pub mod support;
pub mod ui;

pub use domain::{config, hotkeys, model};
pub use platform::{assets, monitors, store_apps, tray};
pub use services::{file_actions as install, process, runtime};
pub use support::logging;
pub use ui::{dialogs, i18n, views};

pub use app::run;
