mod handoff;
mod input;
mod lights;
mod monitor_return;
mod resolved_display;
mod session;
mod settings;
mod status;
mod window;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;

use super::controller::{ControllerManager, LightsOffJob};
use super::display::monitor_watch::{MonitorReturnWatcher, MonitorTarget};
use super::domain::circular_game_list::CircularGameList;
use super::lifecycle::coordinator::LifecycleCoordinator;
use super::support::diagnostics::ControllerDiagnostics;
use super::ui::layouts::LauncherLayoutRegistry;
use super::ui::settings::SettingsDraft;
use super::ControllerConfig;
use crate::launcher_gui::model::LauncherConfig;
use monitor_return::{MonitorReturnState, ReturnRetry};
use resolved_display::ResolvedDisplay;
use session::ReconnectState;
use status::ControllerStatus;
use window::PlacementAttempt;

pub(crate) struct ControllerLauncher {
    launcher_dir: PathBuf,
    games: CircularGameList,
    manager: ControllerManager,
    layouts: LauncherLayoutRegistry,
    display: Option<ResolvedDisplay>,
    open: bool,

    viewport_spawned: bool,

    suspended: bool,
    status: ControllerStatus,
    settings: Option<SettingsDraft>,
    test_pending: bool,
    test_transient: bool,
    lifecycle: LifecycleCoordinator,
    diagnostics: ControllerDiagnostics,
    last_display_refresh: Instant,

    force_display_refresh: bool,

    placement: Option<PlacementAttempt>,
    active_controller_config: Option<ControllerConfig>,

    reconnect: Option<ReconnectState>,
    monitor_watcher: MonitorReturnWatcher,
    monitor_return: MonitorReturnState,

    return_retry: Option<ReturnRetry>,

    ctx: egui::Context,

    lights_off: LightsOffJob,
}

impl ControllerLauncher {
    pub(crate) fn new(
        launcher_dir: PathBuf,
        config: &LauncherConfig,
        log: crate::launcher_gui::logging::LauncherLog,
        ctx: egui::Context,
    ) -> Self {
        let mut this = Self {
            launcher_dir,
            games: CircularGameList::default(),
            manager: ControllerManager::default(),
            layouts: LauncherLayoutRegistry::with_builtin_layouts(),
            display: None,
            open: false,
            viewport_spawned: false,
            suspended: false,
            status: ControllerStatus::Disconnected,
            settings: None,
            test_pending: false,
            test_transient: false,
            lifecycle: LifecycleCoordinator::new(
                config.settings.controller_screen.reopen_after_game_close,
            ),
            diagnostics: ControllerDiagnostics::new(log),
            last_display_refresh: Instant::now(),
            force_display_refresh: false,
            placement: None,
            active_controller_config: None,
            reconnect: None,
            monitor_watcher: MonitorReturnWatcher::new(ctx.clone()),
            monitor_return: MonitorReturnState::Inactive,
            return_retry: None,
            ctx: ctx.clone(),
            lights_off: LightsOffJob::default(),
        };

        this.manager.set_context(ctx);
        this.library_changed(config);
        this
    }

    pub(crate) fn library_changed(&mut self, config: &LauncherConfig) {
        self.games.rebuild(config.games.iter().map(|game| game.id));
        self.configuration_changed(config);
    }

    pub(crate) fn configuration_changed(&mut self, config: &LauncherConfig) {
        self.layouts.library_changed(config);
        self.monitor_watcher
            .configure(&config.settings.controller_screen);
        let target = MonitorTarget::from_config(&config.settings.controller_screen);
        if self.monitor_return.target() != target.as_ref() {
            self.monitor_return = MonitorReturnState::Inactive;
            self.return_retry = None;
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn status(&self) -> String {
        self.status.resolve()
    }

    #[cfg(test)]
    fn viewport_resources(&self) -> (bool, bool, usize) {
        (
            self.open,
            self.display.is_some(),
            self.layouts.resource_count(),
        )
    }

    #[cfg(test)]
    fn resource_counts(&self) -> (usize, usize) {
        self.manager.resource_counts()
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/launcher/mod__tests.rs"]
mod tests;
