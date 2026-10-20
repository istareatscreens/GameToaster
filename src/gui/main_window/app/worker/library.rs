use std::path::{Path, PathBuf};

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use super::plugin_actions::preview::preview_add_game_plugin;
use crate::launcher_gui::config;
use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig};
use crate::plugin::catalog::library::PluginSelection;

impl WorkerContext {
    pub(super) fn add_game(
        &mut self,
        config: LauncherConfig,
        game: GameEntry,
        plugin: Option<PluginSelection>,
    ) -> WorkerResult {
        match plugin {
            None => match add_game_without_plugin(&self.config_path, config, game) {
                Ok(config) => WorkerResult::ConfigCommitted(config),
                Err(message) => WorkerResult::Error {
                    title: t!("errors.add_game_failed").to_string(),
                    message,
                },
            },
            Some(selection) => {
                match preview_add_game_plugin(config, game, selection, &self.plugin_backend) {
                    Ok((pending, preview)) => {
                        self.pending_plugin_action = Some(pending);
                        WorkerResult::PluginActionPreview(preview)
                    }
                    Err(message) => WorkerResult::Error {
                        title: t!("plugins.error.preview_deployment").to_string(),
                        message,
                    },
                }
            }
        }
    }

    pub(super) fn save_config(&self, config: LauncherConfig) -> WorkerResult {
        commit(&self.config_path, &config)
    }

    pub(super) fn save_config_quiet(
        &self,
        config: LauncherConfig,
        retire_backgrounds: Vec<std::path::PathBuf>,
    ) -> WorkerResult {
        match config::save_atomic(&self.config_path, &config) {
            Ok(()) => {
                for previous in retire_backgrounds {
                    if let Err(error) = crate::game_launcher_screen::retire_background_copy(
                        &self.launcher_dir,
                        &previous,
                        config
                            .settings
                            .controller_screen
                            .background_image
                            .as_deref(),
                    ) {
                        self.log.event(
                            "background-retire-failed",
                            &[("path", previous.display().to_string()), ("error", error)],
                        );
                    }
                }
                WorkerResult::ConfigSavedQuiet
            }
            Err(message) => WorkerResult::Error {
                title: t!("errors.save_settings_failed").to_string(),
                message,
            },
        }
    }

    pub(super) fn save_presentation(
        &self,
        mut config: LauncherConfig,
        game_id: GameId,
        label: String,
        source: Option<PathBuf>,
        clear_image: bool,
    ) -> WorkerResult {
        let before = config.clone();
        let old_image = config
            .game(game_id)
            .and_then(|game| game.controller_image.clone());
        let imported = match source.as_deref() {
            Some(source) => {
                crate::game_launcher_screen::import_game_image(&self.launcher_dir, game_id, source)
                    .map(Some)
            }
            None => Ok(None),
        };
        match imported {
            Err(message) => WorkerResult::Error {
                title: t!("errors.import_artwork_failed").to_string(),
                message,
            },
            Ok(imported) => {
                let new_image =
                    imported.or_else(|| (!clear_image).then(|| old_image.clone()).flatten());
                if let Some(game) = config.games.iter_mut().find(|game| game.id == game_id) {
                    game.label = label;
                    game.controller_image = new_image.clone();
                }
                match config::save_atomic(&self.config_path, &config) {
                    Ok(()) => {
                        if let Some((old, CleanupReferences::Committed)) =
                            presentation_cleanup_plan(true, &old_image, &new_image)
                        {
                            if let Err(error) = crate::game_launcher_screen::cleanup_game_image(
                                &self.launcher_dir,
                                &old,
                                &config,
                            ) {
                                self.log
                                    .event("controller-image-cleanup", &[("error", error)]);
                            }
                        }
                        WorkerResult::ConfigCommitted(config)
                    }
                    Err(message) => {
                        if let Some((new_image, CleanupReferences::Original)) =
                            presentation_cleanup_plan(false, &old_image, &new_image)
                        {
                            let _ = crate::game_launcher_screen::cleanup_game_image(
                                &self.launcher_dir,
                                &new_image,
                                &before,
                            );
                        }
                        WorkerResult::Error {
                            title: t!("errors.save_presentation_failed").to_string(),
                            message,
                        }
                    }
                }
            }
        }
    }

    pub(super) fn remove_game(&self, config: LauncherConfig) -> WorkerResult {
        commit(&self.config_path, &config)
    }
}

pub(super) fn add_game_without_plugin(
    config_path: &Path,
    mut config: LauncherConfig,
    game: GameEntry,
) -> Result<LauncherConfig, String> {
    if config.game(game.id).is_some() {
        return Err(t!("errors.game_exists", id => game.id).to_string());
    }
    config.games.push(game);
    config::save_atomic(config_path, &config)?;
    Ok(config)
}

fn commit(path: &std::path::Path, config: &LauncherConfig) -> WorkerResult {
    match config::save_atomic(path, config) {
        Ok(()) => WorkerResult::ConfigCommitted(config.clone()),
        Err(message) => WorkerResult::Error {
            title: t!("errors.save_config_failed").to_string(),
            message,
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanupReferences {
    Original,
    Committed,
}

fn presentation_cleanup_plan(
    committed: bool,
    old_image: &Option<PathBuf>,
    new_image: &Option<PathBuf>,
) -> Option<(PathBuf, CleanupReferences)> {
    if old_image == new_image {
        return None;
    }
    if committed {
        old_image
            .clone()
            .map(|path| (path, CleanupReferences::Committed))
    } else {
        new_image
            .clone()
            .map(|path| (path, CleanupReferences::Original))
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/app/worker/library__tests.rs"]
mod tests;
