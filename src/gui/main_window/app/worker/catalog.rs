use std::path::{Path, PathBuf};

use rust_i18n::t;

use super::context::WorkerContext;
use super::messages::WorkerResult;
use super::plugin_backend::{with_plugin_work, PluginBackend};
use crate::launcher_gui::model::LauncherConfig;
use crate::plugin::catalog::library::PluginDescriptor;

pub(super) fn plugin_list(
    backend: &Result<PluginBackend, String>,
) -> Result<Vec<PluginDescriptor>, String> {
    with_plugin_work(backend, |backend| {
        crate::plugin::catalog::library::list(&backend.catalog)
    })
}

pub(super) fn plugin_catalog_result(
    backend: &Result<PluginBackend, String>,
    notice: Option<String>,
) -> Result<WorkerResult, String> {
    Ok(WorkerResult::PluginCatalog {
        plugins: plugin_list(backend)?,
        notice,
    })
}

pub(super) fn import_plugin(
    backend: &Result<PluginBackend, String>,
    path: &Path,
) -> Result<String, String> {
    with_plugin_work(backend, |backend| {
        crate::plugin::catalog::library::import(&backend.catalog, path)
            .map(|outcome| import_notice(&outcome))
    })
}

pub(super) fn import_notice(outcome: &crate::plugin::catalog::ImportOutcome) -> String {
    match outcome {
        crate::plugin::catalog::ImportOutcome::Imported(revision) => t!(
            "plugins.notice.imported",
            id => revision.plugin_id.as_str(),
            version => revision.plugin_version.to_string(),
            digest => revision.package_sha256.chars().take(12).collect::<String>()
        )
        .to_string(),
        crate::plugin::catalog::ImportOutcome::AlreadyPresent(revision) => t!(
            "plugins.notice.already_present",
            id => revision.plugin_id.as_str(),
            version => revision.plugin_version.to_string(),
            digest => revision.package_sha256.chars().take(12).collect::<String>()
        )
        .to_string(),
    }
}

impl WorkerContext {
    pub(super) fn announce_catalog(&self) {
        match plugin_catalog_result(&self.plugin_backend, None) {
            Ok(result) => {
                let _ = self.results.send(result);
                self.ctx.request_repaint();
            }
            Err(message) => {
                let _ = self.results.send(WorkerResult::Error {
                    title: t!("plugins.error.catalog_start").to_string(),
                    message,
                });
                self.ctx.request_repaint();
            }
        }
    }

    pub(super) fn refresh_plugins(&self) -> WorkerResult {
        match plugin_catalog_result(&self.plugin_backend, None) {
            Ok(result) => result,
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.refresh").to_string(),
                message,
            },
        }
    }

    pub(super) fn import_package(&self, path: PathBuf) -> WorkerResult {
        match import_plugin(&self.plugin_backend, &path) {
            Ok(notice) => match plugin_catalog_result(&self.plugin_backend, Some(notice)) {
                Ok(result) => result,
                Err(message) => WorkerResult::Error {
                    title: t!("plugins.error.imported_refresh").to_string(),
                    message,
                },
            },
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.import").to_string(),
                message,
            },
        }
    }

    pub(super) fn export_plugin(&self, plugin: PluginDescriptor, output: PathBuf) -> WorkerResult {
        match with_plugin_work(&self.plugin_backend, |_backend| {
            crate::plugin::catalog::library::export_revision(&plugin, &output)
        }) {
            Ok(()) => WorkerResult::PluginCatalog {
                plugins: plugin_list(&self.plugin_backend).unwrap_or_default(),
                notice: Some(
                    t!(
                        "plugins.notice.exported",
                        label => plugin.manifest.label.as_str(),
                        version => plugin.revision.plugin_version.to_string(),
                        path => output.display().to_string()
                    )
                    .to_string(),
                ),
            },
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.export").to_string(),
                message,
            },
        }
    }

    pub(super) fn edit_plugin(&self, plugin: PluginDescriptor, manifest: String) -> WorkerResult {
        let outcome = with_plugin_work(&self.plugin_backend, |backend| {
            crate::plugin::catalog::library::import_edited_revision(
                &backend.catalog,
                &self.launcher_dir.join("plugin_drafts"),
                &plugin,
                &manifest,
            )
        });
        match outcome {
            Ok(outcome) => {
                match plugin_catalog_result(&self.plugin_backend, Some(import_notice(&outcome))) {
                    Ok(result) => result,
                    Err(message) => WorkerResult::Error {
                        title: t!("plugins.error.edited_refresh").to_string(),
                        message,
                    },
                }
            }
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.save_revision").to_string(),
                message,
            },
        }
    }

    pub(super) fn remove_catalog_revision(
        &self,
        plugin: PluginDescriptor,
        config: LauncherConfig,
    ) -> WorkerResult {
        let assigned = crate::plugin::catalog::library::assigned_games(
            &plugin,
            config.games.iter().filter_map(|game| {
                game.plugin.as_ref().map(|assignment| {
                    crate::plugin::catalog::library::AssignedRevision {
                        game_id: game.id.0,
                        plugin_id: &assignment.plugin_id,
                        version: &assignment.revision.version,
                        sha256: &assignment.revision.sha256,
                    }
                })
            }),
        );
        let removed = with_plugin_work(&self.plugin_backend, |backend| {
            backend.catalog.remove_revision(&plugin.revision, &assigned)
        });
        match removed {
            Ok(recovery) => match plugin_catalog_result(
                &self.plugin_backend,
                Some(
                    t!(
                        "plugins.notice.moved_recovery",
                        path => recovery.display().to_string()
                    )
                    .to_string(),
                ),
            ) {
                Ok(result) => result,
                Err(message) => WorkerResult::Error {
                    title: t!("plugins.error.removed_refresh").to_string(),
                    message,
                },
            },
            Err(message) => WorkerResult::Error {
                title: t!("plugins.error.remove").to_string(),
                message,
            },
        }
    }
}
