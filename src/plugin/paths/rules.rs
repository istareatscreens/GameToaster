use std::fs;
use std::path::{Path, PathBuf};

use super::normalize::{ends_with_separator, resolve_relative};
use super::verify::{is_reparse, verify_destination, verify_path, VerifiedPath};
use super::{
    AmbiguityKind, DirectoryFallbackChoice, ExternalGrant, ManagedFileKind, PathPolicy,
    PlannedFile, ResolutionAmbiguity, ResolutionChoices, ZeroMatchChoice,
};
use crate::plugin::manifest::{normalize_package_path, CopyType, Manifest, WatchType};
use crate::support::paths::{path_key, paths_equal};

#[derive(Clone, Debug)]
pub(super) struct Rule {
    pub(super) index: usize,
    pub(super) kind: ManagedFileKind,
    pub(super) source_relative: String,
    copy_type: CopyType,
    replace_path: Option<String>,
    pub(super) add_to_config_menu: bool,
}

pub(super) fn manifest_rules(manifest: &Manifest) -> Vec<Rule> {
    let mut rules = Vec::new();
    for entry in &manifest.config_files {
        let index = rules.len();
        rules.push(Rule {
            index,
            kind: ManagedFileKind::Config,
            source_relative: entry.config_file_location.clone(),
            copy_type: entry.config_file_copy_type,
            replace_path: entry.config_file_replace_path.clone(),
            add_to_config_menu: entry.config_add_to_config_menu,
        });
    }
    for entry in &manifest.plugin_files {
        let index = rules.len();
        rules.push(Rule {
            index,
            kind: ManagedFileKind::Plugin,
            source_relative: entry.plugin_file_location.clone(),
            copy_type: entry.plugin_file_copy_type,
            replace_path: entry.plugin_file_replace_path.clone(),
            add_to_config_menu: false,
        });
    }
    rules
}

pub(super) fn resolve_rule(
    rule: &Rule,
    source_basename: &std::ffi::OsStr,
    game_folder: &Path,
    choices: &ResolutionChoices,
    policy: &PathPolicy,
) -> Result<Vec<PathBuf>, String> {
    match rule.copy_type {
        CopyType::GameFolder => Ok(vec![game_folder.join(source_basename)]),
        CopyType::FindAndReplace => {
            let matches = find_named_files(game_folder, source_basename)?;
            resolve_zero_match(rule, matches, game_folder.join(source_basename), choices)
        }
        CopyType::FindAndReplaceInDirectory => {
            let requested = rule
                .replace_path
                .as_deref()
                .ok_or_else(|| format!("rule {} has no replace path", rule.index))?;
            let directory = resolve_relative(game_folder, requested)?;
            verify_destination(&directory, policy)?;
            let matches = if directory.is_dir() {
                find_named_files(&directory, source_basename)?
            } else {
                Vec::new()
            };
            resolve_zero_match(rule, matches, game_folder.join(source_basename), choices)
        }
        CopyType::Absolute => {
            let requested = rule
                .replace_path
                .as_deref()
                .ok_or_else(|| format!("rule {} has no replace path", rule.index))?;
            let exact = resolve_relative(game_folder, requested)?;
            let directory_form = ends_with_separator(requested) || exact.is_dir();
            if !directory_form {
                return Ok(vec![exact]);
            }
            match choices.directory_fallback.get(&rule.index) {
                Some(DirectoryFallbackChoice::NoAction) => Ok(Vec::new()),
                Some(DirectoryFallbackChoice::SearchDirectory) => {
                    if !exact.is_dir() {
                        return Err(format!(
                            "rule {} directory fallback path does not exist: {}",
                            rule.index,
                            exact.display()
                        ));
                    }
                    let matches = find_named_files(&exact, source_basename)?;
                    resolve_zero_match(rule, matches, game_folder.join(source_basename), choices)
                }
                None => Err(format!(
                    "decision required: rule {} absolute destination is a directory ({})",
                    rule.index,
                    exact.display()
                )),
            }
        }
    }
}

fn resolve_zero_match(
    rule: &Rule,
    matches: Vec<PathBuf>,
    fallback: PathBuf,
    choices: &ResolutionChoices,
) -> Result<Vec<PathBuf>, String> {
    if !matches.is_empty() {
        return Ok(matches);
    }
    match choices.zero_match.get(&rule.index) {
        Some(ZeroMatchChoice::CopyToGameFolder) => Ok(vec![fallback]),
        Some(ZeroMatchChoice::NoAction) => Ok(Vec::new()),
        None => Err(format!(
            "decision required: rule {} found no matching destination",
            rule.index
        )),
    }
}

pub(super) fn resolve_launcher(
    manifest: &Manifest,
    files: &[PlannedFile],
    choices: &ResolutionChoices,
) -> Result<(Option<PathBuf>, Option<ResolutionAmbiguity>), String> {
    let Some(raw_launcher) = &manifest.game_launcher else {
        return Ok((None, None));
    };
    let normalized = normalize_package_path(raw_launcher)?;
    let candidates: Vec<PathBuf> = files
        .iter()
        .filter(|file| {
            normalize_package_path(&file.source_relative)
                .is_ok_and(|source| source.eq_ignore_ascii_case(&normalized))
        })
        .map(|file| file.destination.clone())
        .collect();
    choose_preview(
        "gameLauncher",
        AmbiguityKind::Launcher,
        candidates,
        choices.launcher_destination.as_deref(),
    )
}

pub(super) fn resolve_watch(
    manifest: &Manifest,
    game_folder: &Path,
    selected_target: &Path,
    choices: &ResolutionChoices,
) -> Result<(Option<PathBuf>, Option<ResolutionAmbiguity>), String> {
    let watch_type = manifest
        .executable_watch_type
        .unwrap_or(WatchType::GameDirectory);
    let requested = manifest
        .executable_watch
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match watch_type {
        WatchType::GameDirectory => {
            let candidate = match requested {
                Some(value) => resolve_relative(game_folder, value)?,
                None => selected_target.to_path_buf(),
            };
            Ok((Some(candidate), None))
        }
        WatchType::FindInDirectory => {
            let requested = requested
                .ok_or_else(|| "executableWatch is required for find_in_directory".to_string())?;
            let basename = Path::new(requested)
                .file_name()
                .ok_or_else(|| "executableWatch has no filename".to_string())?;
            let candidates = find_named_files(game_folder, basename)?;
            choose_preview(
                "executableWatch",
                AmbiguityKind::WatchTarget,
                candidates,
                choices.watch_destination.as_deref(),
            )
        }
    }
}

fn choose_preview(
    label: &str,
    kind: AmbiguityKind,
    mut candidates: Vec<PathBuf>,
    selected: Option<&Path>,
) -> Result<(Option<PathBuf>, Option<ResolutionAmbiguity>), String> {
    candidates.sort_by_key(|path| path_key(path));
    candidates.dedup_by(|left, right| paths_equal(left, right));
    if let Some(selected) = selected {
        return candidates
            .into_iter()
            .find(|candidate| paths_equal(candidate, selected))
            .map(|selected| (Some(selected), None))
            .ok_or_else(|| format!("{label} selection is not one of the resolved candidates"));
    }
    match candidates.len() {
        0 => Err(format!("{label} did not resolve to an installed file")),
        1 => Ok((Some(candidates.remove(0)), None)),
        _ => Ok((None, Some(ResolutionAmbiguity { kind, candidates }))),
    }
}

pub(super) fn required_external_grant(
    manifest: &Manifest,
    revision_sha256: &str,
    game_id: u64,
    destination: &VerifiedPath,
    choices: &ResolutionChoices,
) -> Option<ExternalGrant> {
    let root = &destination.verified_parent;
    let valid = choices.external_grants.iter().any(|grant| {
        grant.plugin_id == manifest.plugin_id
            && grant.game_id == game_id
            && grant.revision_sha256.eq_ignore_ascii_case(revision_sha256)
            && paths_equal(&grant.destination_root, root)
            && grant.root_identity == Some(destination.parent_identity)
    });
    if valid {
        None
    } else {
        Some(ExternalGrant {
            plugin_id: manifest.plugin_id.clone(),
            game_id,
            revision_sha256: revision_sha256.to_ascii_lowercase(),
            destination_root: destination.verified_parent.clone(),
            root_identity: Some(destination.parent_identity),
        })
    }
}

fn find_named_files(root: &Path, basename: &std::ffi::OsStr) -> Result<Vec<PathBuf>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let wanted = basename.to_string_lossy().to_ascii_lowercase();
    let mut pending = vec![root.to_path_buf()];
    let mut matches = Vec::new();
    while let Some(directory) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?;
        if is_reparse(&metadata) {
            return Err(format!(
                "reparse traversal is forbidden: {}",
                directory.display()
            ));
        }
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
        entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());
        for entry in entries.into_iter().rev() {
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
                format!("failed to inspect {}: {error}", entry.path().display())
            })?;
            if is_reparse(&metadata) {
                return Err(format!(
                    "reparse traversal is forbidden: {}",
                    entry.path().display()
                ));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file()
                && entry.file_name().to_string_lossy().to_ascii_lowercase() == wanted
            {
                matches.push(verify_path(&entry.path())?.final_path);
            }
        }
    }
    matches.sort_by_key(|path| path_key(path));
    Ok(matches)
}
