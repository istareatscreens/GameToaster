use std::fs;
use std::path::{Component, Path, PathBuf};

use super::normalize::{normalize_lexical, path_starts_with_ci};
use super::{FileIdentity, PathPolicy};
use crate::support::paths::paths_equal;

#[derive(Clone, Debug)]
pub(super) struct VerifiedPath {
    pub(super) final_path: PathBuf,
    pub(super) identity: Option<FileIdentity>,
    pub(super) verified_parent: PathBuf,
    pub(super) parent_identity: FileIdentity,
}

pub(super) fn verify_destination(path: &Path, policy: &PathPolicy) -> Result<VerifiedPath, String> {
    reject_unsafe_syntax(path)?;
    let verified = verify_path(path)?;
    reject_protected(&verified.final_path, policy)?;
    Ok(verified)
}

pub(super) fn verify_path(path: &Path) -> Result<VerifiedPath, String> {
    let lexical = normalize_lexical(path)?;
    let mut current = PathBuf::new();
    let mut last_existing: Option<(PathBuf, FileIdentity)> = None;
    let mut destination_identity = None;
    let components: Vec<_> = lexical.components().collect();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());

        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if is_reparse(&metadata) {
                    return Err(format!(
                        "reparse traversal is forbidden: {}",
                        current.display()
                    ));
                }
                let opened = open_verified(&current)?;
                if index + 1 < components.len() && !metadata.is_dir() {
                    return Err(format!(
                        "non-directory path ancestor: {}",
                        current.display()
                    ));
                }
                if index + 1 == components.len() {
                    destination_identity = Some(opened.1);
                }
                last_existing = Some(opened);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(format!("failed to inspect {}: {error}", current.display())),
        }
    }
    let (existing_path, _) = last_existing.ok_or_else(|| {
        format!(
            "path has no verifiable existing ancestor: {}",
            lexical.display()
        )
    })?;
    let final_path = if destination_identity.is_some() {
        existing_path.clone()
    } else {
        let existing_component_count = existing_path.components().count();
        components
            .iter()
            .skip(existing_component_count)
            .fold(existing_path.clone(), |path, component| {
                path.join(component.as_os_str())
            })
    };
    let parent_path = if destination_identity.is_some() {
        final_path
            .parent()
            .ok_or_else(|| format!("destination has no parent: {}", final_path.display()))?
            .to_path_buf()
    } else {
        existing_path
    };
    let (verified_parent, parent_identity) = open_verified(&parent_path)?;
    Ok(VerifiedPath {
        final_path,
        identity: destination_identity,
        verified_parent,
        parent_identity,
    })
}

#[cfg(windows)]
fn open_verified(path: &Path) -> Result<(PathBuf, FileIdentity), String> {
    use std::mem::MaybeUninit;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, GetFinalPathNameByHandleW,
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    let wide = crate::support::win::wide(path);
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
        .map_err(|error| format!("failed to open path safely {}: {error}", path.display()))?
    };
    let result = (|| {
        let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
        unsafe {
            GetFileInformationByHandle(handle, information.as_mut_ptr())
                .map_err(|error| format!("failed to identify path {}: {error}", path.display()))?;
        }
        let information = unsafe { information.assume_init() };
        if information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err(format!(
                "reparse traversal is forbidden: {}",
                path.display()
            ));
        }
        let mut buffer = vec![0_u16; 32_768];
        let length =
            unsafe { GetFinalPathNameByHandleW(handle, &mut buffer, FILE_NAME_NORMALIZED) };
        if length == 0 || length as usize >= buffer.len() {
            return Err(format!(
                "failed to normalize path handle: {}",
                path.display()
            ));
        }
        let mut normalized = String::from_utf16(&buffer[..length as usize])
            .map_err(|_| format!("normalized path is invalid UTF-16: {}", path.display()))?;
        if let Some(stripped) = normalized.strip_prefix(r"\\?\UNC\") {
            normalized = format!(r"\\{stripped}");
        } else if let Some(stripped) = normalized.strip_prefix(r"\\?\") {
            normalized = stripped.to_string();
        }
        Ok((
            PathBuf::from(normalized),
            FileIdentity {
                volume_serial: information.dwVolumeSerialNumber,
                file_index: ((information.nFileIndexHigh as u64) << 32)
                    | information.nFileIndexLow as u64,
            },
        ))
    })();
    unsafe {
        let _ = CloseHandle(handle);
    }
    result
}

#[cfg(not(windows))]
fn open_verified(path: &Path) -> Result<(PathBuf, FileIdentity), String> {
    use std::os::unix::fs::MetadataExt;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to normalize {}: {error}", path.display()))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    Ok((
        canonical,
        FileIdentity {
            volume_serial: metadata.dev() as u32,
            file_index: metadata.ino(),
        },
    ))
}

pub(super) fn reject_unsafe_syntax(path: &Path) -> Result<(), String> {
    let text = path.as_os_str().to_string_lossy();
    if text.starts_with(r"\\?\")
        || text.starts_with(r"\\.\")
        || text.starts_with(r"\??\")
        || text.starts_with(r"\\")
    {
        return Err(format!("device and UNC namespaces are forbidden: {text}"));
    }
    let without_drive = text.get(2..).unwrap_or_default();
    if without_drive.contains(':') {
        return Err(format!("alternate data streams are forbidden: {text}"));
    }
    Ok(())
}

pub(super) fn reject_protected(path: &Path, policy: &PathPolicy) -> Result<(), String> {
    let root = volume_root(path)?;
    if paths_equal(path, &root) {
        return Err(format!(
            "drive-root destinations are forbidden: {}",
            path.display()
        ));
    }
    let mut protected = vec![
        policy.application_root.clone(),
        policy.catalog_root.clone(),
        policy.state_root.clone(),
    ];
    for variable in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"] {
        if let Some(value) = std::env::var_os(variable) {
            protected.push(PathBuf::from(value));
        }
    }
    for denied in protected {
        if denied.is_absolute() && path_starts_with_ci(path, &denied) {
            return Err(format!(
                "destination is inside a protected root {}: {}",
                denied.display(),
                path.display()
            ));
        }
    }
    Ok(())
}

pub(super) fn volume_root(path: &Path) -> Result<PathBuf, String> {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => output.push(prefix.as_os_str()),
            Component::RootDir => {
                output.push("\\");
                return Ok(output);
            }
            _ => break,
        }
    }
    Err(format!("path has no volume root: {}", path.display()))
}

#[cfg(windows)]
pub(super) fn is_reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
pub(super) fn is_reparse(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
