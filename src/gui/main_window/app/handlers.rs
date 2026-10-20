use eframe::egui;
use rust_i18n::t;

use crate::launcher_gui::dialogs::{
    AddDraft, Dialog, DialogEvent, GameSettingsDraft, HotkeyDraft, HotkeyRow, PluginActionDraft,
    PluginChoice, PresentationDraft,
};
use crate::launcher_gui::install;
use crate::launcher_gui::views::{SettingKey, UiAction};

use super::launch_monitor::PendingHide;
use super::worker::{WorkerCommand, WorkerResult};
use super::LauncherApp;

impl LauncherApp {
    pub(super) fn handle_worker_result(&mut self, context: &egui::Context, result: WorkerResult) {
        if !matches!(
            result,
            WorkerResult::Internal
                | WorkerResult::ProgramStarted { .. }
                | WorkerResult::PluginMessage { .. }
                | WorkerResult::PluginOperationEnded { .. }
                | WorkerResult::PluginRuntimeEnded { .. }
                | WorkerResult::PluginInspection { .. }
        ) {
            self.busy = None;
        }
        match result {
            WorkerResult::Internal => {}
            WorkerResult::FilePicked(Some(path)) => {
                match install::validate_selected_executable(&path) {
                    Ok(()) => {
                        if self
                            .config
                            .games
                            .iter()
                            .any(|game| game.selected_target == path)
                        {
                            self.dialog = Some(Dialog::error(
                                t!("errors.game_already_registered"),
                                t!(
                                    "errors.game_already_registered_detail",
                                    path => path.display().to_string()
                                ),
                            ));
                        } else {
                            let label = path
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .map(str::to_string)
                                .unwrap_or_else(|| t!("add.default_label").to_string());
                            self.dialog = Some(Dialog::Add(AddDraft::new(
                                path,
                                label,
                                self.plugins.clone(),
                            )));
                        }
                    }
                    Err(message) => {
                        self.dialog = Some(Dialog::error(t!("errors.invalid_executable"), message))
                    }
                }
            }
            WorkerResult::FilePicked(None) => {}
            WorkerResult::ControllerImagePicked(selected) => {
                if let (Some(path), Some(Dialog::Presentation(draft))) =
                    (selected, self.dialog.as_mut())
                {
                    match load_artwork_preview(context, &path) {
                        Ok(texture) => {
                            draft.select_image(path, texture);
                        }
                        Err(error) => draft.error = Some(error),
                    }
                }
            }
            WorkerResult::ConfigCommitted(config) => {
                self.config = config;
                self.controller.library_changed(&self.config);
                self.hotkeys.reload(&self.config);
                if let Some(tray) = &mut self.tray {
                    if let Err(error) = tray.rebuild(&self.config) {
                        self.dialog = Some(Dialog::error(t!("errors.tray_update_failed"), error));
                    }
                }
            }
            WorkerResult::ConfigSavedQuiet => {}
            WorkerResult::Launched {
                game_id,
                root_pid,
                root_created,
                updated_config,
            } => {
                self.hide_consoles_of(root_pid, root_created);
                if let Some(config) = updated_config {
                    self.config = config;
                    self.controller.library_changed(&self.config);
                    self.hotkeys.reload(&self.config);
                    if let Some(tray) = &mut self.tray {
                        if let Err(error) = tray.rebuild(&self.config) {
                            self.dialog =
                                Some(Dialog::error(t!("errors.tray_update_failed"), error));
                        }
                    }
                }
                self.controller_launch_pending = None;
                if let Err(error) = self.controller.launch_succeeded() {
                    self.log.event(
                        "controller-lifecycle-error",
                        &[("phase", "launch-succeeded".into()), ("error", error)],
                    );
                }
                self.controller_running_game = Some(game_id);
                if self.config.settings.auto_hide_on_launch {
                    let watch = self.config.game(game_id).and_then(|game| {
                        game.watch_target()
                            .filter(|name| !name.trim().is_empty())
                            .map(|name| (name, game.watch_target_path()))
                    });
                    match watch {
                        Some((watch_exe, watch_path)) => {
                            self.pending_hide =
                                Some(PendingHide::new(game_id, watch_exe, watch_path));
                        }
                        None => {
                            context.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            self.main_visible = false;
                        }
                    }
                }
            }
            WorkerResult::ProgramStarted {
                process_id,
                created,
            } => self.hide_consoles_of(process_id, created),
            WorkerResult::PluginMessage { game_id, message } => {
                self.plugin_status
                    .entry(game_id)
                    .or_default()
                    .apply(&message);
            }
            WorkerResult::PluginOperationEnded { game_id } => {
                self.plugin_status.remove(&game_id);
            }
            WorkerResult::PluginRuntimeEnded { game_id, end } => {
                self.plugin_status.remove(&game_id);

                let launch_confirmed = !self
                    .pending_hide
                    .as_ref()
                    .is_some_and(|pending| pending.game_id == game_id)
                    && !self
                        .controller_launch_pending
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == game_id);
                if let Some((title, message)) =
                    super::runtime_end::dialog_for(&end, launch_confirmed)
                {
                    if self
                        .pending_hide
                        .as_ref()
                        .is_some_and(|pending| pending.game_id == game_id)
                    {
                        self.pending_hide = None;
                    }
                    if let Some((_, controller_was_open)) = self.controller_launch_pending.take() {
                        if self.controller.launch_failed() || controller_was_open {
                            self.open_controller();
                        }
                    }
                    self.dialog = Some(Dialog::error(title, message));
                }
            }
            WorkerResult::PluginCatalog { plugins, notice } => {
                self.plugins = plugins.clone();
                if let Some(Dialog::PluginLibrary(draft)) = self.dialog.as_mut() {
                    draft.replace_plugins(plugins);
                    draft.refresh_assignments(&self.config.games);
                    draft.notice = notice;
                } else if let Some(Dialog::Add(draft)) = self.dialog.as_mut() {
                    let choice = &mut draft.plugin_choice;
                    let selected = choice
                        .selected_plugin()
                        .map(|plugin| plugin.revision.package_sha256.clone());
                    choice.plugins = plugins;
                    choice.selected = selected.and_then(|sha| {
                        choice.plugins.iter().position(|plugin| {
                            plugin.revision.package_sha256.eq_ignore_ascii_case(&sha)
                        })
                    });
                } else if let Some(notice) = notice {
                    self.dialog = Some(Dialog::PluginLibrary(
                        crate::launcher_gui::dialogs::PluginLibraryDraft::new(
                            self.plugins.clone(),
                            Some(notice),
                            &self.config.games,
                        ),
                    ));
                }
            }
            WorkerResult::PluginActionPreview(preview) => {
                self.dialog = Some(Dialog::PluginAction(PluginActionDraft::new(preview)));
            }
            WorkerResult::PluginConfigReset {
                game_id,
                restored,
                backups,
            } => {
                let listed = |paths: &[std::path::PathBuf]| {
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let message = if backups.is_empty() {
                    t!("plugins.notice.config_reset", files => listed(&restored)).to_string()
                } else {
                    t!(
                        "plugins.notice.config_reset_backed_up",
                        files => listed(&restored),
                        backups => listed(&backups)
                    )
                    .to_string()
                };
                self.dialog = Some(Dialog::error(
                    t!("plugins.notice.config_reset_title").to_string(),
                    message,
                ));

                if let Some(game) = self.config.game(game_id).cloned() {
                    let _ = self
                        .worker
                        .try_send(WorkerCommand::InspectGamePlugin { game });
                }
            }
            WorkerResult::PluginInspection {
                game_id,
                inspection,
            } => {
                if let Some(Dialog::GameSettings(draft)) = self.dialog.as_mut() {
                    if draft.id == game_id {
                        draft.plugin_inspection = Some(match inspection {
                            Ok(inspection) => {
                                crate::launcher_gui::dialogs::PluginInspectionState::Loaded(
                                    inspection,
                                )
                            }
                            Err(error) => {
                                crate::launcher_gui::dialogs::PluginInspectionState::Failed(error)
                            }
                        });
                    }
                }
            }
            WorkerResult::GameExited(game_id) => self.game_ended(game_id, true),
            WorkerResult::Closed {
                game_id,
                exit_observed,
            } => self.game_ended(game_id, exit_observed),
            WorkerResult::ActiveGamesClosed => {
                if self.controller_running_game.take().is_some() {
                    let should_restore = self.controller.game_exited();
                    if should_restore && !self.closing {
                        self.open_controller();
                    }
                    self.controller.restore_complete();
                }
            }
            WorkerResult::ShutdownComplete(result) => match result {
                Ok(()) => {
                    self.allow_close = true;
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(message) => {
                    self.closing = false;
                    self.dialog = Some(Dialog::error(t!("errors.close_processes_failed"), message));
                }
            },
            WorkerResult::Error { title, message } => {
                if let Some((_, controller_was_open)) = self.controller_launch_pending.take() {
                    if self.controller.launch_failed() || controller_was_open {
                        self.open_controller();
                    }
                }
                self.dialog = Some(Dialog::error(title, message));
            }
        }
    }

    pub(super) fn handle_ui_action(&mut self, context: &egui::Context, action: UiAction) {
        match action {
            UiAction::None => {}
            UiAction::AddGame => {
                self.submit(WorkerCommand::PickExecutable, t!("busy.picking_file"));
            }
            UiAction::Plugins => {
                self.dialog = Some(Dialog::PluginLibrary(
                    crate::launcher_gui::dialogs::PluginLibraryDraft::new(
                        self.plugins.clone(),
                        None,
                        &self.config.games,
                    ),
                ));
                let _ = self.worker.try_send(WorkerCommand::RefreshPlugins);
            }
            UiAction::OpenHotkeys => {
                self.hotkeys.suspend();
                self.dialog = Some(Dialog::Hotkeys(HotkeyDraft {
                    launcher: self.config.settings.launcher_hotkey,
                    launcher_close: self.config.settings.close_game_hotkey,
                    games: self
                        .config
                        .games
                        .iter()
                        .map(|game| HotkeyRow {
                            id: game.id,
                            label: game.label.clone(),
                            launch: game.hotkey,
                            close: game.close_hotkey,
                        })
                        .collect(),
                    capturing: None,
                    error: None,
                }));
            }
            UiAction::Launch(id) => self.launch(id),
            UiAction::SetDefault(default) => {
                let mut candidate = self.config.clone();
                match candidate.set_default(default) {
                    Ok(()) => {
                        self.config = candidate;
                        self.pending_settings_save = Some(self.config.clone());
                    }
                    Err(error) => {
                        self.dialog = Some(Dialog::error(t!("errors.set_default_failed"), error))
                    }
                }
            }
            UiAction::Rename(id) => {
                if let Some(game) = self.config.game(id).cloned() {
                    let preview = game
                        .controller_image
                        .as_ref()
                        .map(|path| load_artwork_preview(context, &self.launcher_dir.join(path)))
                        .transpose();
                    let (preview, error) = match preview {
                        Ok(preview) => (preview, None),
                        Err(error) => (None, Some(error)),
                    };
                    self.dialog = Some(Dialog::Presentation(PresentationDraft::new(
                        game, preview, error,
                    )));
                }
            }
            UiAction::Config(id) => {
                if let Some(game) = self.config.game(id).cloned() {
                    match game.config_targets.len() {
                        0 => {
                            self.dialog = Some(Dialog::error(
                                t!("errors.no_config_files"),
                                t!("errors.no_config_files_detail"),
                            ));
                        }
                        1 => {
                            if let Err(error) = install::open_config_target(&game.config_targets[0])
                            {
                                self.dialog =
                                    Some(Dialog::error(t!("errors.open_config_failed"), error));
                            }
                        }
                        _ => self.dialog = Some(Dialog::ConfigMenu(game)),
                    }
                }
            }
            UiAction::OpenFolder(id) => {
                if let Some(game) = self.config.game(id).cloned() {
                    if let Err(error) = install::reveal_in_explorer(&game.selected_target) {
                        self.dialog = Some(Dialog::error(t!("errors.open_folder_failed"), error));
                    }
                }
            }
            UiAction::CloseGame(id) => self.close_game(id),
            UiAction::ConfigureClose(id) => {
                if let Some(game) = self.config.game(id).cloned() {
                    let inspection_game = game.clone();
                    let plugin = game.plugin.clone();
                    let has_plugin = plugin.is_some();

                    let resolved_watch_name = plugin
                        .as_ref()
                        .and_then(|assignment| assignment.resolved_watch_target.as_ref())
                        .and_then(|path| path.file_name())
                        .and_then(|name| name.to_str())
                        .map(str::to_owned);

                    let watch_exe = game.watch_target().unwrap_or_default();
                    let automatic_watch_name =
                        crate::launcher_gui::model::automatic_watch_name(&game.selected_target);
                    let stored_automatic_watch = game.stored_automatic_watch();
                    self.dialog = Some(Dialog::GameSettings(GameSettingsDraft {
                        id: game.id,
                        label: game.label,
                        watch_exe,
                        close_mode: game.close_mode,
                        close_script: game.close_script,
                        config_targets: game.config_targets,
                        store_picker: None,
                        store_filter: String::new(),
                        plugin_choice: PluginChoice::for_assignment(
                            self.plugins.clone(),
                            plugin.as_ref(),
                        ),
                        plugin,
                        plugin_inspection: has_plugin.then_some(
                            crate::launcher_gui::dialogs::PluginInspectionState::Loading,
                        ),
                        confirm_config_reset: false,
                        resolved_watch_name,
                        automatic_watch_name,
                        stored_automatic_watch,
                    }));
                    if has_plugin {
                        let _ = self.worker.try_send(WorkerCommand::InspectGamePlugin {
                            game: inspection_game,
                        });
                    }
                }
            }
            UiAction::Remove(id) => {
                if let Some(game) = self.config.game(id).cloned() {
                    self.dialog = Some(Dialog::Remove(game));
                }
            }
            UiAction::SetSetting(key, value) => {
                match key {
                    SettingKey::AutoHide => self.config.settings.auto_hide_on_launch = value,
                    SettingKey::HideCommandWindows => {
                        self.config.settings.show_command_windows = !value
                    }
                    SettingKey::LaunchDefaultOnOpen => {
                        self.config.settings.launch_default_on_open = value
                    }
                    SettingKey::PauseOnFailure => self.config.settings.pause_on_failure = value,
                    SettingKey::BlockExitOnCredit => {
                        self.config.settings.block_exit_on_credit = value
                    }
                    SettingKey::ControllerEnabled => {
                        self.config.settings.controller_screen.enabled = value;

                        if !value && self.controller.is_open() {
                            self.close_controller();
                        }
                    }
                }
                self.pending_settings_save = Some(self.config.clone());
            }
            UiAction::SetLaunchOnStartup(enabled) => {
                match crate::launcher_gui::platform::startup::set_enabled(enabled) {
                    Ok(()) => {}
                    Err(error) => {
                        self.dialog =
                            Some(Dialog::error(t!("errors.startup_setting_failed"), error))
                    }
                }
            }
            UiAction::SetLanguage(code) => {
                self.config.settings.language = code.to_string();
                crate::launcher_gui::i18n::apply(code);
                context.request_repaint();

                if let Some(tray) = &mut self.tray {
                    if let Err(error) = tray.rebuild(&self.config) {
                        self.dialog = Some(Dialog::error(t!("errors.tray_update_failed"), error));
                    }
                    if let Err(error) = tray.refresh_tooltip() {
                        self.log.event("tray-tooltip", &[("error", error)]);
                    }
                }
                self.pending_settings_save = Some(self.config.clone());
            }
            UiAction::SetAdapter(adapter) => {
                self.config.settings.adapter = adapter;
                if let Some(index) = adapter {
                    self.move_to_adapter(context, index);
                }

                self.center_frames = self.center_frames.max(4);
                self.pending_settings_save = Some(self.config.clone());
            }
            UiAction::OpenControllerSettings => {
                let startup_enabled = crate::launcher_gui::platform::startup::is_enabled();
                if let Err(error) = self
                    .controller
                    .open_settings(&self.config.settings.controller_screen, startup_enabled)
                {
                    self.dialog = Some(Dialog::error(
                        t!("controller.dialog.settings_open_failed_title").to_string(),
                        error,
                    ));
                }
            }
            UiAction::ToggleControllerScreen => {
                if self.controller.is_open() {
                    self.close_controller();
                } else {
                    self.open_controller();
                }
            }
            UiAction::MinimizeToTray => self.hide_all_to_tray(context),

            UiAction::RequestClose => {
                self.begin_shutdown();
            }
        }
    }

    pub(super) fn handle_dialog_event(&mut self, event: DialogEvent) {
        let closing_hotkeys_without_save = matches!(self.dialog, Some(Dialog::Hotkeys(_)))
            && !matches!(event, DialogEvent::None | DialogEvent::SaveHotkeys { .. });
        if super::pending_notice::closes_notice(&self.dialog, &event) {
            self.config_notice = None;
        }
        match event {
            DialogEvent::None => {}
            DialogEvent::Cancel => self.dialog = None,
            DialogEvent::AddContinue(draft) => {
                self.dialog = None;
                let mut candidate = self.config.clone();
                let plugin = draft.plugin_choice.selection();
                let entry = draft.game_entry(candidate.allocate_id());
                self.submit(
                    WorkerCommand::AddGame {
                        config: candidate,
                        game: entry,
                        plugin,
                    },
                    t!("busy.adding_game"),
                );
            }
            DialogEvent::RemoveGame(game) => {
                self.dialog = None;
                let mut candidate = self.config.clone();
                candidate.remove_game(game.id);
                self.submit(
                    WorkerCommand::RemoveGame { config: candidate },
                    t!("busy.removing_game"),
                );
            }
            DialogEvent::ChooseControllerImage => {
                self.submit(
                    WorkerCommand::PickControllerImage,
                    t!("busy.choosing_artwork"),
                );
            }
            DialogEvent::SavePresentation {
                id,
                label,
                source,
                clear_image,
            } => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::SavePresentation {
                        config: self.config.clone(),
                        game_id: id,
                        label,
                        source,
                        clear_image,
                    },
                    t!("busy.saving_presentation"),
                );
            }
            DialogEvent::SaveHotkeys {
                launcher,
                launcher_close,
                games,
            } => {
                self.dialog = None;
                let mut candidate = self.config.clone();
                candidate.settings.launcher_hotkey = launcher;
                candidate.settings.close_game_hotkey = launcher_close;
                for (id, launch, close) in games {
                    if let Some(game) = candidate.games.iter_mut().find(|game| game.id == id) {
                        game.hotkey = launch;
                        game.close_hotkey = close;
                    }
                }
                self.commit_config(candidate);
            }
            DialogEvent::SaveGameSettings {
                id,
                watch_exe,
                close_mode,
                close_script,
                config_targets,
            } => {
                self.dialog = None;
                let mut candidate = self.config.clone();
                match candidate.games.iter_mut().find(|game| game.id == id) {
                    Some(game) => {
                        game.watch_exe = watch_exe;
                        game.close_mode = close_mode;
                        game.close_script = close_script;
                        game.config_targets = config_targets;
                        self.commit_config(candidate);
                    }
                    None => {
                        self.dialog = Some(Dialog::error(
                            t!("errors.game_not_found"),
                            t!("errors.game_not_found_detail", id => id),
                        ));
                    }
                }
            }
            DialogEvent::ApplyGamePlugin { id, selection } => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::ApplyGamePlugin {
                        config: self.config.clone(),
                        game_id: id,
                        selection,
                    },
                    t!("plugins.operation.apply_game").to_string(),
                );
            }
            DialogEvent::RemoveGamePlugin(id) => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::RemoveGamePlugin {
                        config: self.config.clone(),
                        game_id: id,
                    },
                    t!("plugins.operation.restore_game").to_string(),
                );
            }
            DialogEvent::RepairGamePlugin(id) => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::RepairGamePlugin {
                        config: self.config.clone(),
                        game_id: id,
                    },
                    t!("plugins.operation.repair").to_string(),
                );
            }
            DialogEvent::ResetGamePluginConfig(id) => {
                self.dialog = None;
                if let Some(game) = self.config.game(id).cloned() {
                    self.submit(
                        WorkerCommand::ResetGamePluginConfig { game },
                        t!("plugins.operation.config_reset").to_string(),
                    );
                }
            }
            DialogEvent::ReResolveGamePlugin(id) => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::ReResolveGamePlugin {
                        config: self.config.clone(),
                        game_id: id,
                    },
                    t!("plugins.operation.reresolve").to_string(),
                );
            }
            DialogEvent::OpenConfigTarget(path) => {
                self.dialog = None;
                if let Err(error) = install::open_config_target(&path) {
                    self.dialog = Some(Dialog::error(t!("errors.open_config_failed"), error));
                }
            }
            DialogEvent::RequestStoreApps => {
                let (tx, rx) = std::sync::mpsc::channel();
                if std::thread::Builder::new()
                    .name("gametoaster-store-apps".into())
                    .spawn(move || {
                        let _ = tx.send(crate::launcher_gui::store_apps::list());
                    })
                    .is_ok()
                {
                    self.store_apps_rx = Some(rx);
                }
            }
            DialogEvent::RefreshPlugins => {
                self.submit(
                    WorkerCommand::RefreshPlugins,
                    t!("plugins.operation.refresh").to_string(),
                );
            }
            DialogEvent::ImportPlugin(path) => {
                self.submit(
                    WorkerCommand::ImportPlugin(path),
                    t!("plugins.operation.import").to_string(),
                );
            }
            DialogEvent::ExportPlugin { plugin, output } => {
                self.submit(
                    WorkerCommand::ExportPlugin { plugin, output },
                    t!("plugins.operation.export").to_string(),
                );
            }
            DialogEvent::RevealPlugin(path) => {
                if let Err(error) = install::reveal_in_explorer(&path.join("manifest.toml")) {
                    self.dialog =
                        Some(Dialog::error(t!("plugins.error.reveal").to_string(), error));
                }
            }
            DialogEvent::EditPlugin(plugin) => {
                match crate::launcher_gui::dialogs::ManifestEditorDraft::new(plugin) {
                    Ok(draft) => self.dialog = Some(Dialog::ManifestEditor(Box::new(draft))),
                    Err(error) => {
                        self.dialog =
                            Some(Dialog::error(t!("plugins.error.edit").to_string(), error))
                    }
                }
            }
            DialogEvent::SaveEditedPlugin { plugin, manifest } => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::EditPlugin { plugin, manifest },
                    t!("plugins.operation.validate_revision").to_string(),
                );
            }
            DialogEvent::RemoveCatalogRevision(plugin) => {
                self.submit(
                    WorkerCommand::RemoveCatalogRevision {
                        plugin,
                        config: self.config.clone(),
                    },
                    t!("plugins.operation.move_recovery").to_string(),
                );
            }
            DialogEvent::ConfirmPluginAction { token, approval } => {
                self.dialog = None;
                self.submit(
                    WorkerCommand::ConfirmPluginAction { token, approval },
                    t!("plugins.operation.apply_plan").to_string(),
                );
            }
            DialogEvent::CancelPluginAction(token) => {
                self.dialog = None;
                let _ = self
                    .worker
                    .try_send(WorkerCommand::CancelPluginAction { token });
            }
        }
        if closing_hotkeys_without_save && !matches!(self.dialog, Some(Dialog::Hotkeys(_))) {
            self.hotkeys.reload(&self.config);
        }
    }
}

fn load_artwork_preview(
    context: &egui::Context,
    path: &std::path::Path,
) -> Result<egui::TextureHandle, String> {
    let image = image::open(path)
        .map_err(|error| {
            t!(
                "errors.preview_failed",
                path => path.display().to_string(),
                error => error
            )
            .to_string()
        })?
        .thumbnail(256, 256)
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Ok(context.load_texture(
        format!("controller-artwork-preview:{}", path.display()),
        color,
        egui::TextureOptions::LINEAR,
    ))
}
