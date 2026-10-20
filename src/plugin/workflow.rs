use std::collections::BTreeMap;
use std::path::PathBuf;

use semver::Version;

use super::deployment::{
    AssignmentState, BackupRecord, FirstInstallOriginal, ModificationDecision, TransactionState,
    UserModification,
};
use super::paths::{ExternalGrant, ManagedFileKind, ResolutionAmbiguity};

pub(crate) fn new_preview_token() -> Result<String, String> {
    crate::support::fs::random_hex(16)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowAction {
    AddGame,
    Install,
    Update,
    Change,
    Repair,
    Remove,
}

#[derive(Clone, Debug)]
pub struct ManagedFilePreview {
    pub kind: ManagedFileKind,
    pub source_relative: String,
    pub source_size: Option<u64>,
    pub destination: PathBuf,
    pub destination_existed: bool,
    pub outside_game_folder: bool,
    pub add_to_config_menu: bool,
    pub first_install_original: Option<FirstInstallOriginal>,
    pub backup_chain: Vec<BackupRecord>,
}

#[derive(Clone, Debug)]
pub struct InstallScriptPreview {
    pub source_relative: String,
    pub arguments: Vec<String>,
    pub already_completed: bool,
}

#[derive(Clone, Debug)]
pub struct PluginActionPreview {
    pub token: String,
    pub action: WorkflowAction,
    pub plugin_id: String,
    pub plugin_version: Version,
    pub plugin_md5: String,
    pub plugin_sha256: String,
    pub label: String,
    pub description: String,
    pub selected_target: PathBuf,
    pub resolved_launcher: Option<PathBuf>,
    pub resolved_watch_target: Option<PathBuf>,
    pub files: Vec<ManagedFilePreview>,
    pub install_scripts: Vec<InstallScriptPreview>,
    pub required_external_grants: Vec<ExternalGrant>,
    pub ambiguities: Vec<ResolutionAmbiguity>,
    pub modifications: Vec<UserModification>,
    pub assignment_state: Option<AssignmentState>,
    pub latest_transaction_state: Option<TransactionState>,
    pub transaction_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct PluginActionApproval {
    pub granted_external_roots: Vec<PathBuf>,
    pub launcher_destination: Option<PathBuf>,
    pub watch_destination: Option<PathBuf>,
    pub modification_decisions: BTreeMap<String, ModificationDecision>,
}

#[derive(Clone, Debug)]
pub struct AssignmentInspection {
    pub assignment_id: String,
    pub state: AssignmentState,
    pub latest_transaction_state: Option<TransactionState>,
    pub transaction_count: usize,
    pub receipt_path: PathBuf,
    pub receipt_only: bool,
    pub active_files: Vec<ManagedFileInspection>,
    pub retired_files: Vec<ManagedFileInspection>,
}

#[derive(Clone, Debug)]
pub struct ManagedFileInspection {
    pub kind: ManagedFileKind,
    pub destination: PathBuf,
    pub first_install_original: FirstInstallOriginal,
    pub backup_chain: Vec<BackupRecord>,
}
