pub mod install_scripts;

mod activity;
mod policy;
mod watch;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use activity::*;
pub use policy::*;

use super::catalog::inventory::sha256_file;
use super::catalog::{Catalog, CatalogRevision};
use super::deployment::{
    is_operator_owned, AssignmentLedger, AssignmentState, DeploymentEngine, FailureInjection,
    TransactionState,
};
use super::manifest::{normalize_package_path, Manifest, Runtime};
use super::paths::{build_plan, revalidate_plan, DirectoryFallbackChoice, ZeroMatchChoice};
use crate::support::paths::paths_equal;
use watch::WatchControl;

pub struct LifecycleService {
    catalog: Catalog,
    deployment: DeploymentEngine,
    activity: ActivityGuard,
    watch: Arc<WatchControl>,
}

impl LifecycleService {
    pub fn new(catalog: Catalog, deployment: DeploymentEngine) -> Result<Self, String> {
        let watch = WatchControl::start(catalog.root().to_path_buf())?;
        let activity = ActivityGuard::new(Arc::clone(&watch));
        Ok(Self {
            catalog,
            deployment,
            activity,
            watch,
        })
    }

    pub fn activity(&self) -> &ActivityGuard {
        &self.activity
    }

    #[cfg(test)]
    pub fn watcher_running(&self) -> bool {
        self.watch.is_running()
    }

    pub fn reconcile_for_launch(
        &self,
        request: ReconcileRequest,
    ) -> Result<PreparedLaunch, String> {
        let work = self.activity.try_plugin_operation()?;
        self.deployment.recover_all()?;

        let _catalog_was_dirty = self.watch.take_dirty();
        let revision = self.select_revision(&request)?;
        let raw_manifest =
            fs::read_to_string(revision.root.join("manifest.toml")).map_err(|error| {
                format!(
                    "failed to read selected plugin manifest {}: {error}",
                    revision.root.display()
                )
            })?;
        let manifest = Manifest::parse(&raw_manifest)?;
        if manifest.plugin_id != request.plugin_id
            || manifest.plugin_version != revision.plugin_version
        {
            return Err("selected plugin version identity changed during reconciliation".into());
        }
        let mut choices = request.choices.clone();
        let rule_count = manifest.config_files.len() + manifest.plugin_files.len();
        for index in 0..rule_count {
            choices.zero_match.entry(index).or_insert(
                if request.copy_zero_matches_to_game_folder {
                    ZeroMatchChoice::CopyToGameFolder
                } else {
                    ZeroMatchChoice::NoAction
                },
            );
            choices.directory_fallback.entry(index).or_insert(
                if request.search_directory_destinations {
                    DirectoryFallbackChoice::SearchDirectory
                } else {
                    DirectoryFallbackChoice::NoAction
                },
            );
        }
        let plan = build_plan(
            &revision.root,
            &manifest,
            &revision.package_sha256,
            request.game_id,
            &request.selected_target,
            &choices,
            self.deployment.path_policy(),
        )?;
        let existing = self
            .deployment
            .store()
            .load_if_present(request.game_id, &request.assignment_id)?;
        if !manifest.install_scripts.is_empty()
            && existing.as_ref().is_none_or(|ledger| {
                ledger.plugin_version != revision.plugin_version.to_string()
                    || !ledger
                        .revision_sha256
                        .eq_ignore_ascii_case(&revision.package_sha256)
                    || install_scripts::pending_count(&manifest, ledger) != 0
            })
        {
            return Err(format!(
                "plugin {} {} has install scripts awaiting approval; review this plugin version in Game Settings before launch",
                revision.plugin_id, revision.plugin_version
            ));
        }
        let unchanged = existing
            .as_ref()
            .is_some_and(|ledger| fully_current(ledger, &plan, self.deployment.path_policy()));
        let ledger = if unchanged {
            existing.expect("checked as present")
        } else {
            self.deployment.deploy(
                &request.assignment_id,
                &revision.plugin_version.to_string(),
                &plan,
                &request.modification_decisions,
                FailureInjection::default(),
            )?
        };
        let ledger_reference = self
            .deployment
            .store()
            .ledger_path(request.game_id, &request.assignment_id)?;
        let runtime = resolve_runtime(&manifest, &ledger)?;
        let result = ReconcileResult {
            revision,
            config_targets: ledger.config_targets(),
            runtime,
            ledger,
            ledger_reference,
            changed: !unchanged,
        };
        let activity = work.reserve_game()?;
        Ok(PreparedLaunch {
            reconciliation: result,
            _activity: activity,
        })
    }

    fn select_revision(&self, request: &ReconcileRequest) -> Result<CatalogRevision, String> {
        match &request.policy {
            RevisionPolicy::TrackLatest { allow_prerelease } => self
                .catalog
                .select_latest(&request.plugin_id, *allow_prerelease)?
                .ok_or_else(|| {
                    format!(
                        "no eligible plugin version is available for {}",
                        request.plugin_id
                    )
                }),
            RevisionPolicy::Pinned { version, sha256 } => {
                self.catalog
                    .select_exact(&request.plugin_id, version, sha256)
            }
        }
    }
}

pub fn runtime_executable_candidates<'a>(
    runtime: &Runtime,
    files: impl IntoIterator<Item = (&'a str, &'a Path)>,
) -> Result<Vec<PathBuf>, String> {
    let wanted = normalize_package_path(&runtime.executable)?;
    Ok(files
        .into_iter()
        .filter(|(source_relative, _)| {
            normalize_package_path(source_relative)
                .is_ok_and(|source| source.eq_ignore_ascii_case(&wanted))
        })
        .map(|(_, destination)| destination.to_path_buf())
        .collect())
}

fn resolve_runtime(
    manifest: &Manifest,
    ledger: &AssignmentLedger,
) -> Result<Option<ResolvedRuntime>, String> {
    let Some(runtime) = &manifest.runtime else {
        return Ok(None);
    };
    let matches = runtime_executable_candidates(
        runtime,
        ledger
            .files
            .iter()
            .map(|file| (file.source_relative.as_str(), file.destination.as_path())),
    )?;
    let executable = match matches.as_slice() {
        [executable] if executable.is_file() => executable.clone(),
        [executable] => {
            return Err(format!(
                "deployed runtime executable is missing: {}",
                executable.display()
            ))
        }
        _ => {
            return Err(format!(
                "runtime executable resolved to {} deployed files; expected one",
                matches.len()
            ))
        }
    };
    let is_launcher = ledger
        .resolved_launcher
        .as_ref()
        .is_some_and(|launcher| paths_equal(launcher, &executable));
    Ok(Some(ResolvedRuntime {
        executable,
        operations: runtime.operations.clone(),
        is_launcher,
    }))
}

fn fully_current(
    ledger: &AssignmentLedger,
    plan: &super::paths::DeploymentPlan,
    policy: &super::paths::PathPolicy,
) -> bool {
    if ledger.validate().is_err()
        || ledger.state != AssignmentState::Active
        || ledger.plugin_id != plan.plugin_id
        || ledger.revision_sha256 != plan.revision_sha256
        || ledger.game_id != plan.game_id
        || ledger.game_folder != plan.game_folder
        || ledger.selected_target != plan.selected_target
        || ledger.resolved_launcher != plan.resolved_launcher
        || ledger.resolved_watch_target != plan.resolved_watch_target
        || ledger.files.len() != plan.files.len()
        || ledger.transactions.iter().any(|transaction| {
            !matches!(
                transaction.state,
                TransactionState::Committed | TransactionState::RolledBack
            )
        })
        || revalidate_plan(plan, policy).is_err()
    {
        return false;
    }
    plan.files.iter().all(|planned| {
        ledger
            .files
            .iter()
            .find(|managed| paths_equal(&managed.destination, &planned.destination))
            .is_some_and(|managed| {
                if managed.source_relative != planned.source_relative
                    || managed.add_to_config_menu != planned.add_to_config_menu
                {
                    return false;
                }
                if is_operator_owned(managed.kind) {
                    return managed.destination.is_file();
                }
                managed.last_deployed_sha256 == planned.source_sha256
                    && sha256_file(&managed.destination).ok().as_deref()
                        == Some(planned.source_sha256.as_str())
            })
    })
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/lifecycle/mod__tests.rs"]
mod tests;
