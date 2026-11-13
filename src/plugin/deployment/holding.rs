use std::path::PathBuf;

use super::{is_operator_owned, AssignmentLedger, AssignmentState, FileProvenance};
use crate::plugin::paths::PlannedFile;
use crate::support::paths::paths_equal;

impl FileProvenance {
    fn holds_plugin_bytes(&self, current_sha256: Option<&str>) -> bool {
        current_sha256 == Some(self.last_deployed_sha256.as_str())
    }
}

impl AssignmentLedger {
    pub(crate) fn keeps_instead_of_writing(
        &self,
        planned: &PlannedFile,
        current_sha256: Option<&str>,
    ) -> bool {
        self.state == AssignmentState::Active
            && current_sha256.is_some_and(|current| current != planned.source_sha256)
            && self
                .files
                .iter()
                .find(|managed| paths_equal(&managed.destination, &planned.destination))
                .is_some_and(|managed| !managed.holds_plugin_bytes(current_sha256))
    }

    pub(super) fn leaves_instead_of_restoring(
        &self,
        managed: &FileProvenance,
        current_sha256: Option<&str>,
    ) -> bool {
        !(self.state == AssignmentState::Active && managed.holds_plugin_bytes(current_sha256))
    }

    pub fn kept_by_operator(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|file| !is_operator_owned(file.kind) && file.user_kept_current)
            .map(|file| file.destination.clone())
            .collect()
    }
}
