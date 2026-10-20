use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use getrandom::fill;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

use super::hex;
use super::win::wide;

pub fn random_hex(bytes: usize) -> Result<String, String> {
    let mut value = vec![0_u8; bytes];
    fill(&mut value).map_err(|error| format!("secure random generation failed: {error}"))?;
    Ok(hex::encode(&value))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("path has no filename: {}", path.display()))?
        .to_string_lossy();
    let temporary = parent.join(format!(".{name}.{}.tmp", random_hex(8)?));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("failed to create {}: {error}", temporary.display()))?;
        file.write_all(bytes)
            .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
        file.sync_all()
            .map_err(|error| format!("failed to flush {}: {error}", temporary.display()))?;
        drop(file);
        replace_file(&temporary, path)?;
        sync_directory(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    let source_wide = wide(source);
    let destination_wide = wide(destination);
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| {
            format!(
                "failed to replace {} with {}: {error}",
                destination.display(),
                source.display()
            )
        })
    }
}

fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

pub fn create_staging_directory(root: &Path, prefix: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("failed to create staging root {}: {error}", root.display()))?;
    for _ in 0..32 {
        let candidate = root.join(format!("{prefix}-{}", random_hex(12)?));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create staging directory {}: {error}",
                    candidate.display()
                ))
            }
        }
    }
    Err("could not allocate a unique staging directory".into())
}

#[cfg(test)]
#[path = "../../tests/unit/support/fs__tests.rs"]
mod tests;
