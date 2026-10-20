use std::fs;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use semver::Version;

use super::archive::package_zip;
use super::{Catalog, CatalogRevision, ImportOutcome};
use crate::plugin::manifest::{package_path, Manifest};
use crate::support::fs::{atomic_write, create_staging_directory};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginDescriptor {
    pub revision: CatalogRevision,
    pub manifest: Manifest,
}

impl PluginDescriptor {
    pub fn digest_summary(&self) -> String {
        self.revision.package_sha256.chars().take(12).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginSelection {
    pub plugin_id: String,
    pub version: Version,
    pub sha256: String,
    pub track_latest: bool,
    pub allow_prerelease: bool,
    pub copy_zero_matches_to_game_folder: bool,
    pub search_directory_destinations: bool,
}

impl PluginSelection {
    pub fn exact(descriptor: &PluginDescriptor) -> Self {
        Self {
            plugin_id: descriptor.revision.plugin_id.clone(),
            version: descriptor.revision.plugin_version.clone(),
            sha256: descriptor.revision.package_sha256.clone(),
            track_latest: true,
            allow_prerelease: !descriptor.revision.plugin_version.pre.is_empty(),
            copy_zero_matches_to_game_folder: false,
            search_directory_destinations: false,
        }
    }
}

pub fn list(catalog: &Catalog) -> Result<Vec<PluginDescriptor>, String> {
    catalog
        .list()?
        .into_iter()
        .map(|revision| {
            let path = revision.root.join("manifest.toml");
            let raw = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            let manifest = Manifest::parse(&raw)?;
            Ok(PluginDescriptor { revision, manifest })
        })
        .collect()
}

pub fn import(catalog: &Catalog, path: &Path) -> Result<ImportOutcome, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("zip") => catalog.import_zip(path),
        Some("toml") => catalog.import_manifest(path),
        _ => Err("plugin import must be a .zip archive or manifest.toml".into()),
    }
}

pub fn export_revision(descriptor: &PluginDescriptor, output: &Path) -> Result<(), String> {
    let mut paths = vec!["manifest.toml".to_string()];
    paths.extend(descriptor.manifest.referenced_sources()?);
    paths.sort_by_key(|value| value.to_ascii_lowercase());
    paths.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    package_zip(&descriptor.revision.root, output, &paths)
}

pub fn import_edited_revision(
    catalog: &Catalog,
    draft_root: &Path,
    source: &PluginDescriptor,
    raw_manifest: &str,
) -> Result<ImportOutcome, String> {
    let manifest = Manifest::parse(raw_manifest)?;
    if manifest.plugin_id != source.revision.plugin_id {
        return Err("editing cannot change pluginId; create/import a new plugin instead".into());
    }
    let staging = create_staging_directory(draft_root, "manifest-edit")?;
    let result = (|| {
        atomic_write(
            &staging.join("manifest.toml"),
            manifest.canonical_toml()?.as_bytes(),
        )?;
        for relative in manifest.referenced_sources()? {
            let from = package_path(&source.revision.root, &relative)?;
            if !from.is_file() {
                return Err(format!(
                    "edited manifest references a missing source: {relative}"
                ));
            }
            let to = package_path(&staging, &relative)?;
            let parent = to
                .parent()
                .ok_or_else(|| format!("payload path has no parent: {}", to.display()))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            fs::copy(&from, &to).map_err(|error| {
                format!(
                    "failed to copy edited payload {} to {}: {error}",
                    from.display(),
                    to.display()
                )
            })?;
        }
        catalog.import_manifest(&staging.join("manifest.toml"))
    })();
    if staging.starts_with(draft_root) {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

#[derive(Clone, Copy, Debug)]
pub struct AssignedRevision<'a> {
    pub game_id: u64,
    pub plugin_id: &'a str,
    pub version: &'a Version,
    pub sha256: &'a str,
}

pub fn assigned_games<'a>(
    descriptor: &PluginDescriptor,
    assignments: impl IntoIterator<Item = AssignedRevision<'a>>,
) -> Vec<u64> {
    let mut ids: Vec<_> = assignments
        .into_iter()
        .filter(|assignment| {
            assignment.plugin_id == descriptor.revision.plugin_id
                && *assignment.version == descriptor.revision.plugin_version
                && assignment
                    .sha256
                    .eq_ignore_ascii_case(&descriptor.revision.package_sha256)
        })
        .map(|assignment| assignment.game_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/catalog/library__tests.rs"]
mod tests;
