pub mod archive;
pub mod inventory;
pub mod library;
pub mod watcher;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::{Deserialize, Serialize};

use archive::{extract_bounded, ArchiveLimits};
use inventory::{CanonicalPackage, InventoryEntry};

use crate::support::fs::{atomic_write, create_staging_directory, random_hex};
use crate::support::hex;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CatalogMetadata {
    catalog_version: u32,
    plugin_id: String,
    plugin_version: Version,
    package_md5: String,
    package_sha256: String,

    #[serde(default)]
    imported_at: u64,
    inventory: Vec<InventoryEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogRevision {
    pub plugin_id: String,
    pub plugin_version: Version,
    pub package_md5: String,
    pub package_sha256: String,

    pub imported_at: u64,
    pub root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportOutcome {
    Imported(CatalogRevision),
    AlreadyPresent(CatalogRevision),
}

#[derive(Clone, Debug)]
pub struct Catalog {
    root: PathBuf,
    staging_root: PathBuf,
    quarantine_root: PathBuf,
}

impl Catalog {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|error| format!("failed to create catalog {}: {error}", root.display()))?;
        let staging_root = root.join(".staging");
        let quarantine_root = root.join(".quarantine");
        fs::create_dir_all(&staging_root)
            .map_err(|error| format!("failed to create {}: {error}", staging_root.display()))?;
        fs::create_dir_all(&quarantine_root)
            .map_err(|error| format!("failed to create {}: {error}", quarantine_root.display()))?;
        Ok(Self {
            root,
            staging_root,
            quarantine_root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn import_manifest(&self, manifest_path: &Path) -> Result<ImportOutcome, String> {
        if manifest_path.file_name().and_then(|name| name.to_str()) != Some("manifest.toml") {
            return Err("loose import must select a file named manifest.toml".into());
        }
        let source_root = manifest_path
            .parent()
            .ok_or_else(|| "manifest path has no parent directory".to_string())?;
        let package = CanonicalPackage::from_directory(source_root)?;
        let staging = create_staging_directory(&self.staging_root, "loose")?;
        let result = (|| {
            package.materialize(&staging)?;
            self.commit_staged(&staging)
        })();
        remove_staging(&self.staging_root, &staging);
        result
    }

    pub fn import_zip(&self, archive: &Path) -> Result<ImportOutcome, String> {
        self.import_zip_with_limits(archive, ArchiveLimits::default())
    }

    pub fn import_zip_with_limits(
        &self,
        archive: &Path,
        limits: ArchiveLimits,
    ) -> Result<ImportOutcome, String> {
        let staging = create_staging_directory(&self.staging_root, "zip")?;
        let result = (|| {
            extract_bounded(archive, &staging, limits)?;
            self.commit_staged(&staging)
        })();
        remove_staging(&self.staging_root, &staging);
        result
    }

    fn commit_staged(&self, staging: &Path) -> Result<ImportOutcome, String> {
        let package = CanonicalPackage::from_directory(staging)?;
        let id_dir = encode_plugin_id(&package.manifest.plugin_id);
        let version_dir = self
            .root
            .join(id_dir)
            .join(package.manifest.plugin_version.to_string());
        let destination = version_dir.join(&package.digests.package_sha256);
        if destination.is_dir() {
            let existing = self.verify_revision(&destination)?;
            if existing.package_sha256 == package.digests.package_sha256
                && existing.package_md5 == package.digests.package_md5
            {
                return Ok(ImportOutcome::AlreadyPresent(existing));
            }
            return Err(
                "catalog path exists but validated package inventory does not match".into(),
            );
        }

        let commit = create_staging_directory(&self.staging_root, "commit")?;
        let result = (|| {
            package.materialize(&commit)?;
            let imported_at = self.next_import_time(&package.manifest.plugin_id)?;
            let metadata = metadata_from(&package, imported_at);
            let encoded = toml::to_string_pretty(&metadata)
                .map_err(|error| format!("catalog metadata encode failed: {error}"))?;
            atomic_write(&commit.join("catalog.toml"), encoded.as_bytes())?;
            fs::create_dir_all(&version_dir)
                .map_err(|error| format!("failed to create {}: {error}", version_dir.display()))?;
            fs::rename(&commit, &destination).map_err(|error| {
                format!(
                    "failed to commit plugin version {}: {error}",
                    destination.display()
                )
            })?;
            set_tree_readonly(&destination, true)?;
            self.verify_revision(&destination)
        })();
        if commit.exists() {
            remove_staging(&self.staging_root, &commit);
        }
        result.map(ImportOutcome::Imported)
    }

    pub fn verify_revision(&self, root: &Path) -> Result<CatalogRevision, String> {
        let metadata_path = root.join("catalog.toml");
        let raw = fs::read_to_string(&metadata_path)
            .map_err(|error| format!("failed to read {}: {error}", metadata_path.display()))?;
        let metadata: CatalogMetadata = toml::from_str(&raw)
            .map_err(|error| format!("invalid {}: {error}", metadata_path.display()))?;
        if metadata.catalog_version != 1 {
            return Err(format!(
                "unsupported catalog metadata version {}",
                metadata.catalog_version
            ));
        }
        let package = CanonicalPackage::from_directory(root)?;
        if metadata.plugin_id != package.manifest.plugin_id
            || metadata.plugin_version != package.manifest.plugin_version
            || metadata.package_md5 != package.digests.package_md5
            || metadata.package_sha256 != package.digests.package_sha256
            || metadata.inventory != package.digests.inventory
        {
            return Err(format!(
                "catalog plugin-version integrity mismatch: {}",
                root.display()
            ));
        }
        Ok(revision_from(
            &package,
            root.to_path_buf(),
            metadata.imported_at,
        ))
    }

    fn next_import_time(&self, plugin_id: &str) -> Result<u64, String> {
        let latest = self
            .list()?
            .into_iter()
            .filter(|revision| revision.plugin_id == plugin_id)
            .map(|revision| revision.imported_at)
            .max()
            .unwrap_or(0);
        Ok(now_millis().max(latest.saturating_add(1)))
    }

    pub fn list(&self) -> Result<Vec<CatalogRevision>, String> {
        let mut revisions = Vec::new();
        for id in child_directories(&self.root)? {
            let id_name = id
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if id_name.starts_with('.') {
                continue;
            }
            for version in child_directories(&id)? {
                for digest in child_directories(&version)? {
                    revisions.push(self.verify_revision(&digest)?);
                }
            }
        }
        revisions.sort_by(|left, right| {
            left.plugin_id
                .cmp(&right.plugin_id)
                .then_with(|| left.plugin_version.cmp(&right.plugin_version))
                .then_with(|| left.imported_at.cmp(&right.imported_at))
                .then_with(|| left.package_sha256.cmp(&right.package_sha256))
        });
        Ok(revisions)
    }

    pub fn select_latest(
        &self,
        plugin_id: &str,
        allow_prerelease: bool,
    ) -> Result<Option<CatalogRevision>, String> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|revision| revision.plugin_id == plugin_id)
            .filter(|revision| allow_prerelease || revision.plugin_version.pre.is_empty())
            .max_by(|left, right| {
                left.plugin_version
                    .cmp(&right.plugin_version)
                    .then_with(|| left.imported_at.cmp(&right.imported_at))
                    .then_with(|| left.package_sha256.cmp(&right.package_sha256))
            }))
    }

    pub fn select_exact(
        &self,
        plugin_id: &str,
        version: &Version,
        sha256: &str,
    ) -> Result<CatalogRevision, String> {
        self.list()?
            .into_iter()
            .find(|revision| {
                revision.plugin_id == plugin_id
                    && &revision.plugin_version == version
                    && revision.package_sha256.eq_ignore_ascii_case(sha256)
            })
            .ok_or_else(|| {
                format!("pinned plugin version is unavailable: {plugin_id} {version} {sha256}")
            })
    }

    pub fn quarantine(&self, revision: &Path, reason: &str) -> Result<PathBuf, String> {
        ensure_child(&self.root, revision)?;
        if !revision.exists() {
            return Err(format!(
                "plugin version does not exist: {}",
                revision.display()
            ));
        }
        set_tree_readonly(revision, false)?;
        let destination =
            self.quarantine_root
                .join(format!("{}-{}", sanitize_reason(reason), random_hex(8)?));
        fs::rename(revision, &destination).map_err(|error| {
            format!(
                "failed to quarantine {} as {}: {error}",
                revision.display(),
                destination.display()
            )
        })?;
        Ok(destination)
    }

    pub fn remove_revision(
        &self,
        revision: &CatalogRevision,
        assigned_games: &[u64],
    ) -> Result<PathBuf, String> {
        if !assigned_games.is_empty() {
            return Err(format!(
                "plugin version is assigned to games: {}",
                assigned_games
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        self.quarantine(&revision.root, "removed")
    }
}

fn metadata_from(package: &CanonicalPackage, imported_at: u64) -> CatalogMetadata {
    CatalogMetadata {
        catalog_version: 1,
        plugin_id: package.manifest.plugin_id.clone(),
        plugin_version: package.manifest.plugin_version.clone(),
        package_md5: package.digests.package_md5.clone(),
        package_sha256: package.digests.package_sha256.clone(),
        imported_at,
        inventory: package.digests.inventory.clone(),
    }
}

fn revision_from(package: &CanonicalPackage, root: PathBuf, imported_at: u64) -> CatalogRevision {
    CatalogRevision {
        plugin_id: package.manifest.plugin_id.clone(),
        plugin_version: package.manifest.plugin_version.clone(),
        package_md5: package.digests.package_md5.clone(),
        package_sha256: package.digests.package_sha256.clone(),
        imported_at,
        root,
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

pub fn encode_plugin_id(plugin_id: &str) -> String {
    format!("id-{}", hex::encode(plugin_id.as_bytes()))
}

fn child_directories(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut output = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            output.push(entry.path());
        }
    }
    output.sort();
    Ok(output)
}

fn ensure_child(root: &Path, target: &Path) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", root.display()))?;
    let target = target
        .canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", target.display()))?;
    if target == root || !target.starts_with(&root) {
        return Err(format!("path is outside catalog: {}", target.display()));
    }
    Ok(())
}

fn remove_staging(root: &Path, staging: &Path) {
    if ensure_child(root, staging).is_ok() {
        let _ = set_tree_readonly(staging, false);
        let _ = fs::remove_dir_all(staging);
    }
}

#[allow(clippy::permissions_set_readonly_false)]
fn set_tree_readonly(root: &Path, readonly: bool) -> Result<(), String> {
    let mut paths = vec![root.to_path_buf()];
    let mut directories = Vec::new();
    while let Some(path) = paths.pop() {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("catalog tree contains a link: {}", path.display()));
        }
        if metadata.is_dir() {
            directories.push(path.clone());
            for entry in fs::read_dir(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?
            {
                paths.push(
                    entry
                        .map_err(|error| format!("failed to read {}: {error}", path.display()))?
                        .path(),
                );
            }
        } else {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(readonly);
            fs::set_permissions(&path, permissions).map_err(|error| {
                format!("failed to set permissions on {}: {error}", path.display())
            })?;
        }
    }
    if !readonly {
        for path in directories.into_iter().rev() {
            let mut permissions = fs::metadata(&path)
                .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
                .permissions();
            permissions.set_readonly(false);
            fs::set_permissions(&path, permissions).map_err(|error| {
                format!("failed to set permissions on {}: {error}", path.display())
            })?;
        }
    }
    Ok(())
}

fn sanitize_reason(reason: &str) -> String {
    let value: String = reason
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .take(32)
        .collect();
    if value.is_empty() {
        "unknown".into()
    } else {
        value
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/catalog/mod__tests.rs"]
mod tests;
