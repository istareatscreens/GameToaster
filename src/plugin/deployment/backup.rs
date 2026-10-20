use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::plugin::catalog::inventory::sha256_file;
use crate::support::time::utc_stamp;

pub fn create_dated_backup(source: &Path) -> Result<(PathBuf, String), String> {
    if !source.is_file() {
        return Err(format!("backup source is not a file: {}", source.display()));
    }
    let parent = source
        .parent()
        .ok_or_else(|| format!("backup source has no parent: {}", source.display()))?;
    let filename = source
        .file_name()
        .ok_or_else(|| format!("backup source has no filename: {}", source.display()))?
        .to_string_lossy();
    let stamp = utc_stamp(SystemTime::now())?;
    for suffix in 0_u32..10_000 {
        let name = if suffix == 0 {
            format!("{filename}.{stamp}.original")
        } else {
            format!("{filename}.{stamp}.original.{suffix}")
        };
        let candidate = parent.join(name);
        match copy_create_new(source, &candidate) {
            Ok(hash) => return Ok((candidate, hash)),
            Err(CopyNewError::Collision) => continue,
            Err(CopyNewError::Failure(error)) => return Err(error),
        }
    }
    Err(format!(
        "could not allocate collision-safe backup name for {}",
        source.display()
    ))
}

enum CopyNewError {
    Collision,
    Failure(String),
}

fn copy_create_new(source: &Path, destination: &Path) -> Result<String, CopyNewError> {
    let mut input = fs::File::open(source).map_err(|error| {
        CopyNewError::Failure(format!("failed to read {}: {error}", source.display()))
    })?;
    let mut output = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(CopyNewError::Collision)
        }
        Err(error) => {
            return Err(CopyNewError::Failure(format!(
                "failed to create backup {}: {error}",
                destination.display()
            )))
        }
    };
    let result = (|| {
        std::io::copy(&mut input, &mut output).map_err(|error| {
            format!("failed to write backup {}: {error}", destination.display())
        })?;
        output
            .flush()
            .and_then(|_| output.sync_all())
            .map_err(|error| {
                format!("failed to flush backup {}: {error}", destination.display())
            })?;
        let source_hash = sha256_file(source)?;
        let backup_hash = sha256_file(destination)?;
        if source_hash != backup_hash {
            return Err(format!(
                "backup verification mismatch for {}",
                destination.display()
            ));
        }
        Ok(backup_hash)
    })();
    result.map_err(CopyNewError::Failure)
}

pub fn stage_bytes_adjacent(
    destination: &Path,
    bytes: &[u8],
    token: &str,
) -> Result<PathBuf, String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("destination has no parent: {}", destination.display()))?;
    create_directories_without_reparse(parent)?;
    let filename = destination
        .file_name()
        .ok_or_else(|| format!("destination has no filename: {}", destination.display()))?
        .to_string_lossy();
    for counter in 0_u32..1000 {
        let stage = parent.join(format!(".{filename}.GameToaster-{token}-{counter}.stage"));
        let mut output = match OpenOptions::new().create_new(true).write(true).open(&stage) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create stage {}: {error}",
                    stage.display()
                ))
            }
        };
        let result = (|| {
            output
                .write_all(bytes)
                .map_err(|error| format!("failed to write stage {}: {error}", stage.display()))?;
            output
                .flush()
                .and_then(|_| output.sync_all())
                .map_err(|error| format!("failed to flush stage {}: {error}", stage.display()))?;
            let mut verification = Vec::with_capacity(bytes.len());
            fs::File::open(&stage)
                .and_then(|mut file| file.read_to_end(&mut verification))
                .map_err(|error| format!("failed to verify stage {}: {error}", stage.display()))?;
            if verification != bytes {
                return Err(format!("stage verification mismatch: {}", stage.display()));
            }
            Ok(stage.clone())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&stage);
        }
        return result;
    }
    Err(format!(
        "could not allocate adjacent stage for {}",
        destination.display()
    ))
}

fn create_directories_without_reparse(path: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if !metadata.is_dir() || is_reparse(&metadata) {
                    return Err(format!(
                        "unsafe or non-directory path ancestor: {}",
                        current.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|error| {
                    format!("failed to create directory {}: {error}", current.display())
                })?;
            }
            Err(error) => return Err(format!("failed to inspect {}: {error}", current.display())),
        }
    }
    Ok(())
}

pub fn move_to_recovery(
    source: &Path,
    recovery_root: &Path,
    token: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(recovery_root)
        .map_err(|error| format!("failed to create {}: {error}", recovery_root.display()))?;
    let filename = source
        .file_name()
        .ok_or_else(|| format!("recovery source has no filename: {}", source.display()))?
        .to_string_lossy();
    for counter in 0_u32..1000 {
        let destination = recovery_root.join(format!("{filename}.{token}.{counter}.recovery"));
        if destination.exists() {
            continue;
        }
        move_new(source, &destination)?;
        return Ok(destination);
    }
    Err(format!(
        "could not allocate recovery destination for {}",
        source.display()
    ))
}

#[cfg(windows)]
pub fn replace_existing(destination: &Path, staged: &Path) -> Result<(), String> {
    use crate::support::win::wide;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    let destination_wide = wide(destination);
    let staged_wide = wide(staged);
    unsafe {
        ReplaceFileW(
            PCWSTR(destination_wide.as_ptr()),
            PCWSTR(staged_wide.as_ptr()),
            PCWSTR::null(),
            REPLACEFILE_WRITE_THROUGH,
            None,
            None,
        )
        .map_err(|error| {
            format!(
                "failed to replace {} from {}: {error}",
                destination.display(),
                staged.display()
            )
        })
    }
}

#[cfg(not(windows))]
pub fn replace_existing(destination: &Path, staged: &Path) -> Result<(), String> {
    fs::rename(staged, destination).map_err(|error| {
        format!(
            "failed to replace {} from {}: {error}",
            destination.display(),
            staged.display()
        )
    })
}

#[cfg(windows)]
pub fn move_new(source: &Path, destination: &Path) -> Result<(), String> {
    use crate::support::win::wide;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};

    if destination.exists() {
        return Err(format!(
            "refusing to overwrite recovery path: {}",
            destination.display()
        ));
    }
    let source_wide = wide(source);
    let destination_wide = wide(destination);
    unsafe {
        MoveFileExW(
            PCWSTR(source_wide.as_ptr()),
            PCWSTR(destination_wide.as_ptr()),
            MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| {
            format!(
                "failed to move {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })
    }
}

#[cfg(not(windows))]
pub fn move_new(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err(format!(
            "refusing to overwrite recovery path: {}",
            destination.display()
        ));
    }
    fs::rename(source, destination).map_err(|error| {
        format!(
            "failed to move {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })
}

#[cfg(windows)]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
fn is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/deployment/backup__tests.rs"]
mod tests;
