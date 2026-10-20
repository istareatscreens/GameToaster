use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use md5::Md5;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::plugin::manifest::{package_path, validate_package_file, Manifest};
use crate::support::hex;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InventoryEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PackageDigests {
    pub package_md5: String,
    pub package_sha256: String,
    pub inventory: Vec<InventoryEntry>,
}

#[derive(Clone, Debug)]
pub struct CanonicalPackage {
    pub manifest: Manifest,
    pub canonical_manifest: String,
    pub digests: PackageDigests,
    pub files: Vec<(String, PathBuf)>,
}

impl CanonicalPackage {
    pub fn from_directory(root: &Path) -> Result<Self, String> {
        let manifest_path = root.join("manifest.toml");
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
        let manifest = Manifest::parse(&raw)?;
        Self::from_manifest(root, manifest)
    }

    pub fn from_manifest(root: &Path, manifest: Manifest) -> Result<Self, String> {
        let canonical_manifest = manifest.canonical_toml()?;
        let mut content = BTreeMap::<String, (Vec<u8>, PathBuf)>::new();
        content.insert(
            "manifest.toml".into(),
            (
                canonical_manifest.as_bytes().to_vec(),
                root.join("manifest.toml"),
            ),
        );
        for source in manifest.referenced_sources()? {
            let path = validate_package_file(root, &source)?;
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            if content.insert(source.clone(), (bytes, path)).is_some() {
                return Err(format!("duplicate normalized package path '{source}'"));
            }
        }

        let mut md5 = Md5::new();
        let mut sha256 = Sha256::new();
        let mut inventory = Vec::with_capacity(content.len());
        for (path, (bytes, _)) in &content {
            feed(&mut md5, path, bytes);
            feed(&mut sha256, path, bytes);
            inventory.push(InventoryEntry {
                path: path.clone(),
                size: bytes.len() as u64,
                sha256: hex::encode(&Sha256::digest(bytes)),
            });
        }
        let files = content
            .into_iter()
            .map(|(path, (_, source))| (path, source))
            .collect();
        Ok(Self {
            manifest,
            canonical_manifest,
            digests: PackageDigests {
                package_md5: hex::encode(&md5.finalize()),
                package_sha256: hex::encode(&sha256.finalize()),
                inventory,
            },
            files,
        })
    }

    pub fn materialize(&self, destination: &Path) -> Result<(), String> {
        fs::create_dir_all(destination).map_err(|error| {
            format!(
                "failed to create package destination {}: {error}",
                destination.display()
            )
        })?;
        for (relative, source) in &self.files {
            let destination_path = package_path(destination, relative)?;
            let parent = destination_path
                .parent()
                .ok_or_else(|| format!("invalid package path '{relative}'"))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            if relative == "manifest.toml" {
                fs::write(&destination_path, self.canonical_manifest.as_bytes()).map_err(
                    |error| format!("failed to write {}: {error}", destination_path.display()),
                )?;
            } else {
                fs::copy(source, &destination_path).map_err(|error| {
                    format!(
                        "failed to copy {} to {}: {error}",
                        source.display(),
                        destination_path.display()
                    )
                })?;
            }
        }
        Ok(())
    }
}

fn feed<D: Digest>(digest: &mut D, path: &str, bytes: &[u8]) {
    digest.update((path.len() as u64).to_le_bytes());
    digest.update(path.as_bytes());
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    std::io::copy(&mut file, &mut DigestWriter(&mut digest))
        .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
    Ok(hex::encode(&digest.finalize()))
}

struct DigestWriter<'a, D: Digest>(&'a mut D);

impl<D: Digest> std::io::Write for DigestWriter<'_, D> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/catalog/inventory__tests.rs"]
mod tests;
