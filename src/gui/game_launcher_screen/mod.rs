mod controller;
mod display;
mod domain;
mod launcher;
mod lifecycle;
mod support;
mod surface;
mod ui;

pub(crate) use display::target::{configured_display_available, describe_monitors};
pub(crate) use domain::command::ControllerLauncherAction;
pub(crate) use domain::model::{
    ControllerConfig, ControllerScreenConfig, LauncherXAction, MonitorCalibration,
    OrientationProfile,
};
pub(crate) use launcher::ControllerLauncher;
pub(crate) use lifecycle::coordinator::LaunchSource as ControllerLaunchSource;

use crate::launcher_gui::model::{GameId, LauncherConfig};

pub(crate) fn validate_controller_config(config: &ControllerConfig) -> Result<(), String> {
    controller::AdapterRegistry::with_default_adapters().validate(config)
}

pub(crate) fn default_controller_config(driver_id: &str) -> Result<ControllerConfig, String> {
    controller::AdapterRegistry::with_default_adapters().default_config(driver_id)
}

pub(crate) fn validate_screen_config(config: &ControllerScreenConfig) -> Result<(), String> {
    config.validate()?;
    validate_controller_config(&config.controller)?;
    Ok(ui::layouts::LauncherLayoutRegistry::with_builtin_layouts()
        .validate(&config.launcher_layout)?)
}

pub(crate) fn import_game_image(
    launcher_dir: &std::path::Path,
    game_id: GameId,
    source: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    support::assets::import_image(launcher_dir, game_id, source)
}

pub(crate) fn cleanup_game_image(
    launcher_dir: &std::path::Path,
    candidate: &std::path::Path,
    config: &LauncherConfig,
) -> Result<bool, String> {
    support::assets::remove_if_unreferenced(
        launcher_dir,
        candidate,
        config
            .games
            .iter()
            .filter_map(|game| game.controller_image.clone()),
    )
}

pub(crate) fn resolve_background_image(
    launcher_dir: &std::path::Path,
    picked: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    match picked {
        None => None,
        Some(path) if support::assets::is_launcher_owned_background(&path) => Some(path),
        Some(source) => match support::assets::import_background(launcher_dir, &source) {
            Ok(relative) => Some(relative),
            Err(_) => Some(source),
        },
    }
}

pub(crate) fn background_copy_to_retire(
    previous: Option<&std::path::Path>,
    resolved: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    let previous = previous.filter(|path| support::assets::is_launcher_owned_background(path))?;
    let replaced = match resolved {
        None => true,
        Some(resolved) => {
            support::assets::is_launcher_owned_background(resolved) && resolved != previous
        }
    };
    replaced.then(|| previous.to_path_buf())
}

pub(crate) fn retire_background_copy(
    launcher_dir: &std::path::Path,
    previous: &std::path::Path,
    current: Option<&std::path::Path>,
) -> Result<bool, String> {
    support::assets::remove_if_unreferenced(
        launcher_dir,
        previous,
        current.map(std::path::Path::to_path_buf),
    )
}

pub(crate) fn migrate_background_on_load(
    launcher_dir: &std::path::Path,
    current: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    match current {
        Some(source)
            if source.is_absolute() && !support::assets::is_launcher_owned_background(&source) =>
        {
            match support::assets::import_background(launcher_dir, &source) {
                Ok(relative) => Some(relative),
                Err(_) => Some(source),
            }
        }
        other => other,
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/gui/game_launcher_screen/mod__boundary_tests.rs"]
mod boundary_tests;
