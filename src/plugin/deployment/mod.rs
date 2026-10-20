mod backup;
mod change;
mod deploy;
mod execution;
mod identity;
mod model;
mod planning;
mod preflight;
mod provenance;
mod recovery;
mod remove;
mod reset;
mod store;

use std::path::PathBuf;

#[cfg(test)]
pub(crate) use model::RecoveryFile;
pub(crate) use model::RecoveryReceipt;
pub use model::{
    AssignmentLedger, AssignmentState, BackupRecord, FailurePoint, FileCommitState, FileProvenance,
    FirstInstallOriginal, InstallScriptReceipt, InstallScriptRunState, ModificationDecision,
    ModificationPurpose, Transaction, TransactionFileAction, TransactionKind, TransactionState,
    UserModification,
};
pub use store::LedgerStore;

pub use identity::decision_key;

use crate::plugin::paths::{ManagedFileKind, PathPolicy};
use crate::support::fs::random_hex;

pub(crate) fn is_operator_owned(kind: ManagedFileKind) -> bool {
    matches!(kind, ManagedFileKind::Config)
}

#[derive(Clone, Copy, Debug)]
pub struct FailureInjection {
    pub fail: FailurePoint,
    pub rollback_at: Option<usize>,
}

impl Default for FailureInjection {
    fn default() -> Self {
        Self {
            fail: FailurePoint::Never,
            rollback_at: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DeploymentEngine {
    store: LedgerStore,
    path_policy: PathPolicy,
}

#[derive(Clone, Debug)]
pub struct ConfigurationReset {
    pub restored: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct PluginChangeResult {
    pub removed_assignment: AssignmentLedger,
    pub active_assignment: AssignmentLedger,
}

#[derive(Clone, Debug)]
pub struct PluginChangeError {
    pub new_plugin_error: String,
    pub old_plugin_reinstalled: bool,
    pub recovery_error: Option<String>,
}

impl std::fmt::Display for PluginChangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.old_plugin_reinstalled {
            write!(
                formatter,
                "replacement plugin failed and the previous plugin was reinstalled: {}",
                self.new_plugin_error
            )
        } else {
            write!(
                formatter,
                "replacement plugin failed and previous-plugin recovery requires attention: {}{}",
                self.new_plugin_error,
                self.recovery_error
                    .as_deref()
                    .map(|error| format!("; {error}"))
                    .unwrap_or_default()
            )
        }
    }
}

impl DeploymentEngine {
    pub fn new(state_root: impl Into<PathBuf>, path_policy: PathPolicy) -> Result<Self, String> {
        Ok(Self {
            store: LedgerStore::new(state_root)?,
            path_policy,
        })
    }

    pub fn store(&self) -> &LedgerStore {
        &self.store
    }

    pub fn path_policy(&self) -> &PathPolicy {
        &self.path_policy
    }

    pub fn new_assignment_id(&self) -> Result<String, String> {
        random_hex(16)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/deployment/mod__tests.rs"]
mod tests;
