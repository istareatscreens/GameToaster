use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::support::time::utc_stamp;

pub(super) fn keep(path: &Path, what: Option<&str>) -> Result<PathBuf, String> {
    keep_at(path, what, SystemTime::now())
}

fn keep_at(path: &Path, what: Option<&str>, time: SystemTime) -> Result<PathBuf, String> {
    let fail = |error: String| format!("failed to back up {}: {error}", path.display());
    let original = fs::read(path).map_err(|error| fail(error.to_string()))?;
    let folder = path
        .parent()
        .ok_or_else(|| fail("the file has no folder".into()))?;
    let name = path
        .file_name()
        .ok_or_else(|| fail("the path names no file".into()))?
        .to_string_lossy();
    let stamp = utc_stamp(time).map_err(fail)?;
    let label = what.map(|what| format!(".{what}")).unwrap_or_default();
    for attempt in 0_u32..1_000 {
        let collision = if attempt == 0 {
            String::new()
        } else {
            format!(".{attempt}")
        };
        let copy = folder.join(format!("{name}{label}.{stamp}{collision}.bak"));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&copy) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(fail(format!("{}: {error}", copy.display()))),
        };
        let written = file.write_all(&original).and_then(|()| file.sync_all());
        drop(file);
        let verified =
            written
                .map_err(|error| error.to_string())
                .and_then(|()| match fs::read(&copy) {
                    Ok(read) if read == original => Ok(()),
                    Ok(_) => Err("the copy differs from the file".to_string()),
                    Err(error) => Err(error.to_string()),
                });
        if let Err(error) = verified {
            let _ = fs::remove_file(&copy);
            return Err(fail(format!("{}: {error}", copy.display())));
        }
        return Ok(copy);
    }
    Err(fail("no free backup name".into()))
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/domain/config/migration/backup__tests.rs"]
mod tests;
