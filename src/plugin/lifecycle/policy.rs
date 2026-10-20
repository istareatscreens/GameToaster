use std::collections::BTreeMap;
use std::path::PathBuf;

use semver::Version;

use super::activity::ManagedGameActivity;
use crate::plugin::catalog::CatalogRevision;
use crate::plugin::deployment::{AssignmentLedger, ModificationDecision};
use crate::plugin::manifest::Operation;
use crate::plugin::paths::ResolutionChoices;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionPolicy {
    TrackLatest { allow_prerelease: bool },
    Pinned { version: Version, sha256: String },
}

#[derive(Clone, Debug)]
pub struct ReconcileRequest {
    pub assignment_id: String,
    pub plugin_id: String,
    pub game_id: u64,
    pub selected_target: PathBuf,
    pub policy: RevisionPolicy,
    pub choices: ResolutionChoices,
    pub copy_zero_matches_to_game_folder: bool,
    pub search_directory_destinations: bool,
    pub modification_decisions: BTreeMap<String, ModificationDecision>,
}

#[derive(Clone, Debug)]
pub struct ReconcileResult {
    pub revision: CatalogRevision,
    pub ledger: AssignmentLedger,
    pub ledger_reference: PathBuf,
    pub config_targets: Vec<PathBuf>,
    pub runtime: Option<ResolvedRuntime>,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedRuntime {
    pub executable: PathBuf,
    pub operations: Vec<Operation>,
    pub is_launcher: bool,
}

pub struct PreparedLaunch {
    pub reconciliation: ReconcileResult,
    pub(super) _activity: ManagedGameActivity,
}

impl PreparedLaunch {
    #[cfg(test)]
    pub fn finish(self) {
        drop(self);
    }
}
