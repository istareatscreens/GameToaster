use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use zip::ZipArchive;

use crate::plugin::manifest::normalize_package_path;

#[derive(Clone, Copy, Debug)]
pub struct ArchiveLimits {
    pub max_entries: usize,
    pub max_entry_size: u64,
    pub max_total_size: u64,
    pub max_nesting: usize,
    pub max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            max_entry_size: 256 * 1024 * 1024,
            max_total_size: 1024 * 1024 * 1024,
            max_nesting: 16,
            max_compression_ratio: 200,
        }
    }
}

pub fn extract_bounded(
    archive_path: &Path,
    staging: &Path,
    limits: ArchiveLimits,
) -> Result<(), String> {
    if staging.exists() {
        if staging
            .read_dir()
            .map_err(|error| format!("failed to inspect {}: {error}", staging.display()))?
            .next()
            .is_some()
        {
            return Err(format!(
                "archive staging directory must be empty: {}",
                staging.display()
            ));
        }
    } else {
        fs::create_dir_all(staging)
            .map_err(|error| format!("failed to create {}: {error}", staging.display()))?;
    }
    let file = fs::File::open(archive_path)
        .map_err(|error| format!("failed to open {}: {error}", archive_path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("invalid ZIP archive: {error}"))?;
    if archive.len() > limits.max_entries {
        return Err(format!(
            "ZIP contains {} entries; limit is {}",
            archive.len(),
            limits.max_entries
        ));
    }

    let canonical_staging = staging
        .canonicalize()
        .map_err(|error| format!("failed to resolve staging directory: {error}"))?;
    let mut names = BTreeSet::new();
    let mut total_size = 0_u64;
    let mut manifest_count = 0_usize;

    for index in 0..archive.len() {
        let encrypted_name = {
            let entry = archive
                .by_index_raw(index)
                .map_err(|error| format!("failed to inspect raw ZIP entry {index}: {error}"))?;
            entry.encrypted().then(|| entry.name().to_string())
        };
        if let Some(name) = encrypted_name {
            return Err(format!("encrypted ZIP entry is not allowed: {name}"));
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to inspect ZIP entry {index}: {error}"))?;
        let raw_name = entry.name().replace('\\', "/");
        let directory = raw_name.ends_with('/');
        let trimmed = raw_name.trim_end_matches('/');
        if trimmed.is_empty() {
            return Err("ZIP contains an empty root entry".into());
        }
        let normalized = normalize_package_path(trimmed)
            .map_err(|error| format!("unsafe ZIP entry '{}': {error}", entry.name()))?;
        if normalized.split('/').count() > limits.max_nesting {
            return Err(format!(
                "ZIP entry exceeds nesting limit {}: {normalized}",
                limits.max_nesting
            ));
        }
        if !names.insert(normalized.to_ascii_lowercase()) {
            return Err(format!(
                "ZIP contains duplicate or case-colliding path: {normalized}"
            ));
        }
        if normalized == "manifest.toml" {
            manifest_count += 1;
        } else if normalized.eq_ignore_ascii_case("manifest.toml") {
            return Err("root manifest must be named exactly 'manifest.toml'".into());
        }

        let mode_type = entry.unix_mode().map(|mode| mode & 0o170000);
        if mode_type.is_some_and(|kind| kind != 0 && kind != 0o100000 && kind != 0o040000) {
            return Err(format!(
                "link or special ZIP entry is forbidden: {normalized}"
            ));
        }
        if directory && mode_type == Some(0o100000) {
            return Err(format!(
                "ZIP file/directory metadata conflicts: {normalized}"
            ));
        }
        if !directory && mode_type == Some(0o040000) {
            return Err(format!(
                "ZIP file/directory metadata conflicts: {normalized}"
            ));
        }

        if entry.size() > limits.max_entry_size {
            return Err(format!(
                "ZIP entry exceeds {} bytes: {normalized}",
                limits.max_entry_size
            ));
        }
        total_size = total_size
            .checked_add(entry.size())
            .ok_or_else(|| "ZIP expanded-size overflow".to_string())?;
        if total_size > limits.max_total_size {
            return Err(format!(
                "ZIP expanded size exceeds {} bytes",
                limits.max_total_size
            ));
        }
        if entry.size() > 0
            && (entry.compressed_size() == 0
                || entry.size() / entry.compressed_size().max(1) > limits.max_compression_ratio)
        {
            return Err(format!(
                "ZIP entry compression ratio exceeds {}: {normalized}",
                limits.max_compression_ratio
            ));
        }

        let destination = normalized
            .split('/')
            .fold(staging.to_path_buf(), |path, part| path.join(part));
        if directory {
            fs::create_dir_all(&destination)
                .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
            continue;
        }
        let parent = destination
            .parent()
            .ok_or_else(|| format!("invalid ZIP destination: {}", destination.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|error| format!("failed to resolve {}: {error}", parent.display()))?;
        if !canonical_parent.starts_with(&canonical_staging) {
            return Err(format!("ZIP entry escapes staging: {normalized}"));
        }
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(|error| format!("refusing to overwrite {}: {error}", destination.display()))?;
        let expected_size = entry.size();
        let mut bounded = (&mut entry).take(expected_size.saturating_add(1));
        let copied = std::io::copy(&mut bounded, &mut output)
            .map_err(|error| format!("failed to extract {normalized}: {error}"))?;
        if copied != expected_size {
            return Err(format!(
                "ZIP entry size mismatch for {normalized}: expected {}, wrote {copied}",
                expected_size
            ));
        }
        output
            .flush()
            .and_then(|_| output.sync_all())
            .map_err(|error| format!("failed to flush {}: {error}", destination.display()))?;
    }
    if manifest_count != 1 {
        return Err(format!(
            "ZIP must contain exactly one root manifest.toml; found {manifest_count}"
        ));
    }
    Ok(())
}

pub fn package_zip(source_root: &Path, output_path: &Path, paths: &[String]) -> Result<(), String> {
    let parent = output_path
        .parent()
        .ok_or_else(|| format!("ZIP output has no parent: {}", output_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .map_err(|error| format!("failed to create {}: {error}", output_path.display()))?;
    let mut zip = zip::ZipWriter::new(output);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let mut sorted = paths.to_vec();
    sorted.sort();
    sorted.dedup();
    for relative in sorted {
        let normalized = normalize_package_path(&relative)?;
        let source = normalized
            .split('/')
            .fold(source_root.to_path_buf(), |path, part| path.join(part));
        let bytes = fs::read(&source)
            .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
        zip.start_file(normalized, options)
            .map_err(|error| format!("failed to start ZIP entry: {error}"))?;
        zip.write_all(&bytes)
            .map_err(|error| format!("failed to write ZIP entry: {error}"))?;
    }
    let output = zip
        .finish()
        .map_err(|error| format!("failed to finish ZIP: {error}"))?;
    output
        .sync_all()
        .map_err(|error| format!("failed to flush {}: {error}", output_path.display()))
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/catalog/archive__tests.rs"]
mod tests;
