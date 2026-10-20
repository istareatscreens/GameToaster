use std::fs;
use std::path::Path;

use super::normalize::{path_starts_with_ci, validate_sha256};
use super::rules::{
    manifest_rules, required_external_grant, resolve_launcher, resolve_rule, resolve_watch,
};
use super::verify::{verify_destination, verify_path};
use super::{
    AmbiguityKind, DeploymentPlan, ExternalGrant, PathPolicy, PlanPreview, PlannedFile,
    ResolutionChoices,
};
use crate::plugin::catalog::inventory::sha256_file;
use crate::plugin::manifest::{validate_package_file, Manifest};
use crate::support::paths::{path_key, paths_equal};

pub fn build_plan(
    package_root: &Path,
    manifest: &Manifest,
    revision_sha256: &str,
    game_id: u64,
    selected_target: &Path,
    choices: &ResolutionChoices,
    policy: &PathPolicy,
) -> Result<DeploymentPlan, String> {
    let preview = build_plan_preview(
        package_root,
        manifest,
        revision_sha256,
        game_id,
        selected_target,
        choices,
        policy,
    )?;
    if preview.is_ready() {
        return Ok(preview.plan);
    }
    if let Some(grant) = preview.required_external_grants.first() {
        return Err(format!(
            "external destination requires a plugin-version access grant: {}",
            grant.destination_root.display()
        ));
    }
    if let Some(ambiguity) = preview.ambiguities.first() {
        let label = match ambiguity.kind {
            AmbiguityKind::Launcher => "gameLauncher",
            AmbiguityKind::WatchTarget => "executableWatch",
        };
        return Err(format!(
            "{label} resolved to {} files; an explicit selection is required",
            ambiguity.candidates.len()
        ));
    }
    Ok(preview.plan)
}

pub fn build_plan_preview(
    package_root: &Path,
    manifest: &Manifest,
    revision_sha256: &str,
    game_id: u64,
    selected_target: &Path,
    choices: &ResolutionChoices,
    policy: &PathPolicy,
) -> Result<PlanPreview, String> {
    manifest.validate()?;
    validate_sha256(revision_sha256)?;
    if !selected_target.is_absolute() || !selected_target.is_file() {
        return Err(format!(
            "selected target must be an existing absolute file: {}",
            selected_target.display()
        ));
    }
    let selected = verify_path(selected_target)?;
    let game_folder_source = selected_target.parent().ok_or_else(|| {
        format!(
            "selected target has no parent: {}",
            selected_target.display()
        )
    })?;
    let game_folder = verify_path(game_folder_source)?;
    if game_folder.identity.is_none() {
        return Err("selected game folder does not exist".into());
    }
    let game_folder_path = game_folder.final_path;
    let rules = manifest_rules(manifest);
    let mut files = Vec::new();
    let mut required_external_grants = Vec::new();

    for rule in &rules {
        let source = validate_package_file(package_root, &rule.source_relative)?;
        let source_sha256 = sha256_file(&source)?;
        let source_size = fs::metadata(&source)
            .map_err(|error| format!("failed to inspect {}: {error}", source.display()))?
            .len();
        let basename = source
            .file_name()
            .ok_or_else(|| format!("plugin source has no filename: {}", source.display()))?;
        let destinations = resolve_rule(rule, basename, &game_folder_path, choices, policy)?;
        for destination in destinations {
            let verified = verify_destination(&destination, policy)?;
            let outside = !path_starts_with_ci(&verified.final_path, &game_folder_path);
            if outside {
                if let Some(grant) =
                    required_external_grant(manifest, revision_sha256, game_id, &verified, choices)
                {
                    if !required_external_grants
                        .iter()
                        .any(|candidate: &ExternalGrant| candidate == &grant)
                    {
                        required_external_grants.push(grant);
                    }
                }
            }
            files.push(PlannedFile {
                rule_index: rule.index,
                kind: rule.kind,
                source_relative: rule.source_relative.clone(),
                source: source.clone(),
                source_sha256: source_sha256.clone(),
                source_size,
                destination: verified.final_path,
                destination_identity: verified.identity,
                verified_parent: verified.verified_parent,
                parent_identity: verified.parent_identity,
                destination_existed: verified.identity.is_some(),
                outside_game_folder: outside,
                add_to_config_menu: rule.add_to_config_menu,
            });
        }
    }

    files.sort_by_key(|entry| path_key(&entry.destination));
    for pair in files.windows(2) {
        if path_key(&pair[0].destination) == path_key(&pair[1].destination) {
            return Err(format!(
                "multiple plugin rules resolve to the same destination: {}",
                pair[0].destination.display()
            ));
        }
    }

    let (resolved_launcher, launcher_ambiguity) = resolve_launcher(manifest, &files, choices)?;
    let (resolved_watch_target, watch_ambiguity) =
        resolve_watch(manifest, &game_folder_path, &selected.final_path, choices)?;
    let ambiguities = launcher_ambiguity
        .into_iter()
        .chain(watch_ambiguity)
        .collect();

    Ok(PlanPreview {
        plan: DeploymentPlan {
            plan_version: 1,
            plugin_id: manifest.plugin_id.clone(),
            revision_sha256: revision_sha256.to_ascii_lowercase(),
            game_id,
            game_folder: game_folder_path,
            selected_target: selected.final_path,
            resolved_launcher,
            resolved_watch_target,
            files,
        },
        required_external_grants,
        ambiguities,
    })
}

pub fn revalidate_plan(plan: &DeploymentPlan, policy: &PathPolicy) -> Result<(), String> {
    for file in &plan.files {
        let current = verify_destination(&file.destination, policy)?;
        if !paths_equal(&current.final_path, &file.destination)
            || current.identity != file.destination_identity
            || current.parent_identity != file.parent_identity
            || !paths_equal(&current.verified_parent, &file.verified_parent)
        {
            return Err(format!(
                "destination changed after preview; refusing commit: {}",
                file.destination.display()
            ));
        }
        let hash = sha256_file(&file.source)?;
        let size = fs::metadata(&file.source)
            .map_err(|error| format!("failed to inspect {}: {error}", file.source.display()))?
            .len();
        if hash != file.source_sha256 || size != file.source_size {
            return Err(format!(
                "plugin source changed after preview: {}",
                file.source.display()
            ));
        }
    }
    Ok(())
}

pub fn rebind_destination_leaves(
    plan: &DeploymentPlan,
    policy: &PathPolicy,
) -> Result<DeploymentPlan, String> {
    let mut rebound = plan.clone();
    for file in &mut rebound.files {
        let source_metadata = fs::metadata(&file.source).map_err(|error| {
            format!(
                "failed to revalidate source {}: {error}",
                file.source.display()
            )
        })?;
        if !source_metadata.is_file()
            || source_metadata.len() != file.source_size
            || sha256_file(&file.source)? != file.source_sha256
        {
            return Err(format!(
                "plugin source changed after preview: {}",
                file.source.display()
            ));
        }
        let verified = verify_destination(&file.destination, policy)?;
        if !paths_equal(&verified.final_path, &file.destination)
            || verified.verified_parent != file.verified_parent
            || verified.parent_identity != file.parent_identity
        {
            return Err(format!(
                "destination parent/grant identity changed after preview: {}",
                file.destination.display()
            ));
        }
        file.destination_identity = verified.identity;
        file.destination_existed = file.destination.is_file();
    }
    Ok(rebound)
}
