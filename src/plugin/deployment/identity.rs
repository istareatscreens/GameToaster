use std::path::Path;

use crate::plugin::catalog::inventory::sha256_file;
use crate::support::paths::path_key;

pub fn decision_key(path: &Path) -> String {
    path_key(path)
}

pub(super) fn hash_optional(path: &Path) -> Result<Option<String>, String> {
    if path.is_file() {
        sha256_file(path).map(Some)
    } else if path.exists() {
        Err(format!(
            "managed destination is not a file: {}",
            path.display()
        ))
    } else {
        Ok(None)
    }
}
