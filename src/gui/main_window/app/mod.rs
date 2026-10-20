use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use rust_i18n::t;

use crate::game_launcher_screen::ControllerLauncher;
use crate::gui::common::theme;
use crate::launcher_gui::dialogs::Dialog;
use crate::launcher_gui::hotkeys::HotkeyService;
use crate::launcher_gui::logging::LauncherLog;
use crate::launcher_gui::model::LauncherConfig;
use crate::launcher_gui::tray::TrayController;

pub(crate) mod console_hider;
mod frame;
mod handlers;
mod launch_monitor;
mod pending_notice;
mod run;
mod runtime_end;
mod startup_screen;
mod state;
mod worker;

pub use run::run;

use console_hider::ConsoleHider;
use launch_monitor::PendingHide;
use worker::Worker;

struct LauncherApp {
    launcher_dir: PathBuf,
    config: LauncherConfig,
    controller: ControllerLauncher,
    worker: Worker,
    tray: Option<TrayController>,
    hotkeys: HotkeyService,
    center_frames: u32,
    pending_hide: Option<PendingHide>,
    pending_console_hide: Option<ConsoleHider>,
    pending_settings_save: Option<LauncherConfig>,

    pending_background_retire: Vec<PathBuf>,
    store_apps_rx: Option<Receiver<Result<Vec<crate::launcher_gui::store_apps::StoreApp>, String>>>,
    dialog: Option<Dialog>,

    config_notice: Option<crate::launcher_gui::config::Notice>,
    busy: Option<String>,
    startup_launch_attempted: bool,
    controller_startup_attempted: bool,
    controller_launch_pending: Option<(crate::launcher_gui::model::GameId, bool)>,
    controller_running_game: Option<crate::launcher_gui::model::GameId>,
    main_visible: bool,

    started_in_tray: bool,

    minimized_restore_main: bool,
    closing: bool,
    allow_close: bool,

    game_state: Box<dyn crate::gui::common::game_state::GameStateSource>,
    plugin_status:
        BTreeMap<crate::launcher_gui::model::GameId, crate::plugin::runtime::RuntimeStatus>,
    plugins: Vec<crate::plugin::catalog::library::PluginDescriptor>,
    log: LauncherLog,
}

impl LauncherApp {
    fn new(
        creation: &eframe::CreationContext<'_>,
        launcher_dir: PathBuf,
        mut config: LauncherConfig,
        config_notice: Option<crate::launcher_gui::config::Notice>,
        log: LauncherLog,
        start_in_tray: bool,
        main_visible: bool,
    ) -> Self {
        let background_before = config.settings.controller_screen.background_image.clone();
        config.settings.controller_screen.background_image =
            crate::game_launcher_screen::migrate_background_on_load(
                &launcher_dir,
                config.settings.controller_screen.background_image.take(),
            );
        let migrated_settings_save = (config.settings.controller_screen.background_image
            != background_before)
            .then(|| config.clone());

        theme::install_fonts(&creation.egui_ctx);

        crate::launcher_gui::i18n::apply(crate::launcher_gui::i18n::resolved_code(
            &config.settings.language,
        ));
        theme::apply(&creation.egui_ctx);
        let (tray, tray_error) =
            match TrayController::new(&launcher_dir, &config, creation.egui_ctx.clone()) {
                Ok(tray) => (Some(tray), None),
                Err(error) => (None, Some(error)),
            };
        let mut hotkeys = HotkeyService::new(creation.egui_ctx.clone(), log.clone());
        hotkeys.reload(&config);
        let controller = ControllerLauncher::new(
            launcher_dir.clone(),
            &config,
            log.clone(),
            creation.egui_ctx.clone(),
        );
        let game_state = crate::gui::common::game_state::GameStateHub::default();
        Self {
            worker: Worker::start(
                launcher_dir.clone(),
                log.clone(),
                creation.egui_ctx.clone(),
                game_state.clone(),
            ),
            launcher_dir,
            config,
            controller,
            tray,
            hotkeys,
            center_frames: 8,
            pending_hide: None,
            pending_console_hide: None,
            pending_settings_save: migrated_settings_save,
            pending_background_retire: Vec::new(),
            store_apps_rx: None,
            dialog: tray_error.map(|error| Dialog::error(t!("errors.tray_init_failed"), error)),
            config_notice,
            busy: None,
            startup_launch_attempted: false,
            controller_startup_attempted: false,
            controller_launch_pending: None,
            controller_running_game: None,
            main_visible,
            started_in_tray: start_in_tray,
            minimized_restore_main: false,
            closing: false,
            allow_close: false,
            game_state: Box::new(game_state),
            plugin_status: BTreeMap::new(),
            plugins: Vec::new(),
            log,
        }
    }
}
