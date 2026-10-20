use std::path::Path;

use crate::plugin::paths::PlanPreview;
use crate::plugin::workflow::{ManagedFilePreview, PluginActionPreview, WorkflowAction};
use crate::support::paths::paths_equal;

#[allow(clippy::too_many_arguments)]
pub(super) fn plugin_action_summary(
    token: String,
    action: WorkflowAction,
    revision: &crate::plugin::catalog::CatalogRevision,
    manifest: &crate::plugin::manifest::Manifest,
    selected_target: &Path,
    preview: Option<&PlanPreview>,
    ledger: Option<&crate::plugin::deployment::AssignmentLedger>,
    modifications: Vec<crate::plugin::deployment::UserModification>,
) -> PluginActionPreview {
    let files = match preview {
        Some(preview) => preview
            .plan
            .files
            .iter()
            .map(|planned| {
                let prior = ledger.and_then(|ledger| {
                    ledger
                        .files
                        .iter()
                        .find(|file| paths_equal(&file.destination, &planned.destination))
                });
                ManagedFilePreview {
                    kind: planned.kind,
                    source_relative: planned.source_relative.clone(),
                    source_size: Some(planned.source_size),
                    destination: planned.destination.clone(),
                    destination_existed: planned.destination_existed,
                    outside_game_folder: planned.outside_game_folder,
                    add_to_config_menu: planned.add_to_config_menu,
                    first_install_original: prior.map(|file| file.first_install_original.clone()),
                    backup_chain: prior
                        .map(|file| file.backup_chain.clone())
                        .unwrap_or_default(),
                }
            })
            .collect(),
        None => ledger
            .map(|ledger| {
                ledger
                    .files
                    .iter()
                    .map(|file| ManagedFilePreview {
                        kind: file.kind,
                        source_relative: file.source_relative.clone(),
                        source_size: None,
                        destination: file.destination.clone(),
                        destination_existed: file.destination.is_file(),
                        outside_game_folder: !path_is_within(
                            &file.destination,
                            &ledger.game_folder,
                        ),
                        add_to_config_menu: file.add_to_config_menu,
                        first_install_original: Some(file.first_install_original.clone()),
                        backup_chain: file.backup_chain.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    let (resolved_launcher, resolved_watch_target, required_external_grants, ambiguities) =
        match preview {
            Some(preview) => (
                preview.plan.resolved_launcher.clone(),
                preview.plan.resolved_watch_target.clone(),
                preview.required_external_grants.clone(),
                preview.ambiguities.clone(),
            ),
            None => (
                ledger.and_then(|ledger| ledger.resolved_launcher.clone()),
                ledger.and_then(|ledger| ledger.resolved_watch_target.clone()),
                Vec::new(),
                Vec::new(),
            ),
        };
    let install_scripts = if action == WorkflowAction::Remove {
        Vec::new()
    } else {
        manifest
            .install_scripts
            .iter()
            .enumerate()
            .map(
                |(index, script)| crate::plugin::workflow::InstallScriptPreview {
                    source_relative: script.install_script_location.clone(),
                    arguments: script.install_script_arguments.clone(),
                    already_completed: ledger.is_some_and(|ledger| {
                        ledger.install_script_receipts.iter().any(|receipt| {
                            receipt.plugin_version == revision.plugin_version.to_string()
                                && receipt
                                    .revision_sha256
                                    .eq_ignore_ascii_case(&revision.package_sha256)
                                && receipt.script_index == index as u32
                                && receipt.state
                                    == crate::plugin::deployment::InstallScriptRunState::Succeeded
                        })
                    }),
                },
            )
            .collect()
    };
    PluginActionPreview {
        token,
        action,
        plugin_id: revision.plugin_id.clone(),
        plugin_version: revision.plugin_version.clone(),
        plugin_md5: revision.package_md5.clone(),
        plugin_sha256: revision.package_sha256.clone(),
        label: manifest.label.clone(),
        description: manifest.description.clone(),
        selected_target: selected_target.to_path_buf(),
        resolved_launcher,
        resolved_watch_target,
        files,
        install_scripts,
        required_external_grants,
        ambiguities,
        modifications,
        assignment_state: ledger.map(|ledger| ledger.state),
        latest_transaction_state: ledger
            .and_then(|ledger| ledger.transactions.last())
            .map(|transaction| transaction.state),
        transaction_count: ledger.map_or(0, |ledger| ledger.transactions.len()),
    }
}

pub(super) fn path_is_within(path: &Path, root: &Path) -> bool {
    let path: Vec<_> = path
        .components()
        .map(|value| value.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    let root: Vec<_> = root
        .components()
        .map(|value| value.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    root.len() <= path.len()
        && root
            .iter()
            .zip(path.iter())
            .all(|(left, right)| left == right)
}
