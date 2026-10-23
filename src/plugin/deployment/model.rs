use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::plugin::paths::{DeploymentPlan, ManagedFileKind};
use crate::support::paths::{path_key, paths_equal};

pub const LEDGER_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentState {
    Active,
    Removed,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallScriptRunState {
    Started,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InstallScriptReceipt {
    pub plugin_version: String,
    pub revision_sha256: String,
    pub script_index: u32,
    pub source_relative: String,
    pub source_sha256: String,
    pub attempts: u32,
    pub state: InstallScriptRunState,
    pub exit_code: Option<i32>,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionKind {
    Install,
    Update,
    Remove,
    Change,
    Repair,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    Planned,
    Staged,
    BackedUp,
    Committing,
    Committed,
    RollingBack,
    RolledBack,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileCommitState {
    Planned,
    Staged,
    BackedUp,
    Committed,
    RolledBack,
    RecoveryRequired,
    KeptByUser,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionFileAction {
    Write,
    RestoreOriginal,
    RemoveToRecovery,
    KeepCurrent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FirstInstallOriginal {
    Existing { backup: PathBuf, sha256: String },
    Absent,

    Preserved { sha256: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BackupRecord {
    pub path: PathBuf,
    pub sha256: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FileProvenance {
    pub kind: ManagedFileKind,
    pub source_relative: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub first_install_original: FirstInstallOriginal,
    pub backup_chain: Vec<BackupRecord>,
    pub last_deployed_sha256: String,
    pub add_to_config_menu: bool,
    #[serde(default)]
    pub user_kept_current: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TransactionFile {
    pub action: TransactionFileAction,
    pub destination: PathBuf,
    pub source: Option<PathBuf>,
    pub expected_sha256: Option<String>,
    pub staged_path: Option<PathBuf>,
    pub preexisting: bool,
    pub pre_hash: Option<String>,
    pub transaction_backup: Option<PathBuf>,
    pub recovery_path: Option<PathBuf>,
    pub state: FileCommitState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Transaction {
    pub transaction_id: String,
    pub kind: TransactionKind,
    pub state: TransactionState,
    pub files: Vec<TransactionFile>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AssignmentLedger {
    pub ledger_version: u32,
    pub assignment_id: String,
    pub plugin_id: String,
    pub plugin_version: String,
    pub revision_sha256: String,
    pub game_id: u64,
    pub game_folder: PathBuf,
    pub selected_target: PathBuf,
    pub resolved_launcher: Option<PathBuf>,
    pub resolved_watch_target: Option<PathBuf>,
    pub state: AssignmentState,
    pub files: Vec<FileProvenance>,
    pub retired_files: Vec<FileProvenance>,
    pub transactions: Vec<Transaction>,
    #[serde(default)]
    pub install_script_receipts: Vec<InstallScriptReceipt>,
}

impl AssignmentLedger {
    pub fn new(assignment_id: String, plugin_version: String, plan: &DeploymentPlan) -> Self {
        Self {
            ledger_version: LEDGER_VERSION,
            assignment_id,
            plugin_id: plan.plugin_id.clone(),
            plugin_version,
            revision_sha256: plan.revision_sha256.clone(),
            game_id: plan.game_id,
            game_folder: plan.game_folder.clone(),
            selected_target: plan.selected_target.clone(),
            resolved_launcher: plan.resolved_launcher.clone(),
            resolved_watch_target: plan.resolved_watch_target.clone(),
            state: AssignmentState::Active,
            files: Vec::new(),
            retired_files: Vec::new(),
            transactions: Vec::new(),
            install_script_receipts: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ledger_version != LEDGER_VERSION {
            return Err(format!(
                "unsupported assignment ledger version {}",
                self.ledger_version
            ));
        }
        if self.assignment_id.len() != 32
            || !self
                .assignment_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("assignment ID must be 128-bit hexadecimal".into());
        }
        if self.plugin_id.trim().is_empty() || self.revision_sha256.len() != 64 {
            return Err("assignment identity is incomplete".into());
        }
        if !self.game_folder.is_absolute() || !self.selected_target.is_absolute() {
            return Err("assignment paths must be absolute".into());
        }
        let mut destinations = std::collections::BTreeSet::new();
        for file in &self.files {
            if !destinations.insert(path_key(&file.destination)) {
                return Err(format!(
                    "duplicate managed destination in ledger: {}",
                    file.destination.display()
                ));
            }
        }
        let mut scripts = std::collections::BTreeSet::new();
        for receipt in &self.install_script_receipts {
            semver::Version::parse(&receipt.plugin_version)
                .map_err(|error| format!("invalid install-script plugin version: {error}"))?;
            validate_hash("install-script plugin package", &receipt.revision_sha256)?;
            validate_hash("install-script source", &receipt.source_sha256)?;
            if receipt.source_relative.trim().is_empty() || receipt.attempts == 0 {
                return Err("install-script receipt is incomplete".into());
            }
            let key = format!(
                "{}\0{}\0{}",
                receipt.plugin_version.to_ascii_lowercase(),
                receipt.revision_sha256.to_ascii_lowercase(),
                receipt.script_index
            );
            if !scripts.insert(key) {
                return Err("duplicate install-script receipt identity".into());
            }
        }
        Ok(())
    }

    pub fn config_targets(&self) -> Vec<PathBuf> {
        if self.state != AssignmentState::Active {
            return Vec::new();
        }
        let mut paths: Vec<_> = self
            .files
            .iter()
            .filter(|file| file.add_to_config_menu)
            .map(|file| file.destination.clone())
            .collect();
        paths.sort_by_key(|path| path_key(path));
        paths.dedup_by(|left, right| paths_equal(left, right));
        paths
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RecoveryReceipt {
    pub receipt_version: u32,
    pub assignment_id: String,
    pub plugin_id: String,
    pub plugin_version: String,
    pub revision_sha256: String,
    pub game_id: u64,
    pub transaction_state: Option<TransactionState>,
    pub files: Vec<RecoveryFile>,
    #[serde(default)]
    pub install_script_receipts: Vec<InstallScriptReceipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RecoveryFile {
    #[serde(default = "managed_plugin_kind")]
    pub kind: ManagedFileKind,
    pub destination: PathBuf,
    pub first_install_original: FirstInstallOriginal,
    pub last_deployed_sha256: String,
    pub backups: Vec<BackupRecord>,
}

fn managed_plugin_kind() -> ManagedFileKind {
    ManagedFileKind::Plugin
}

impl From<&AssignmentLedger> for RecoveryReceipt {
    fn from(ledger: &AssignmentLedger) -> Self {
        Self {
            receipt_version: 1,
            assignment_id: ledger.assignment_id.clone(),
            plugin_id: ledger.plugin_id.clone(),
            plugin_version: ledger.plugin_version.clone(),
            revision_sha256: ledger.revision_sha256.clone(),
            game_id: ledger.game_id,
            transaction_state: ledger.transactions.last().map(|value| value.state),
            files: ledger
                .files
                .iter()
                .chain(ledger.retired_files.iter())
                .map(|file| RecoveryFile {
                    kind: file.kind,
                    destination: file.destination.clone(),
                    first_install_original: file.first_install_original.clone(),
                    last_deployed_sha256: file.last_deployed_sha256.clone(),
                    backups: file.backup_chain.clone(),
                })
                .collect(),
            install_script_receipts: ledger.install_script_receipts.clone(),
        }
    }
}

impl RecoveryReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.receipt_version != 1 {
            return Err(format!(
                "unsupported recovery receipt version {}",
                self.receipt_version
            ));
        }
        if self.assignment_id.len() != 32
            || !self
                .assignment_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("recovery receipt assignment ID must be 128-bit hexadecimal".into());
        }
        if self.plugin_id.trim().is_empty() {
            return Err("recovery receipt plugin ID is empty".into());
        }
        semver::Version::parse(&self.plugin_version)
            .map_err(|error| format!("invalid recovery receipt plugin version: {error}"))?;
        validate_hash("recovery receipt revision", &self.revision_sha256)?;

        let mut destinations = std::collections::BTreeSet::new();
        for file in &self.files {
            if !file.destination.is_absolute() {
                return Err(format!(
                    "recovery receipt destination is not absolute: {}",
                    file.destination.display()
                ));
            }
            if !destinations.insert(path_key(&file.destination)) {
                return Err(format!(
                    "duplicate recovery receipt destination: {}",
                    file.destination.display()
                ));
            }
            validate_hash("last deployed file", &file.last_deployed_sha256)?;
            if let FirstInstallOriginal::Existing { backup, sha256 } = &file.first_install_original
            {
                validate_backup_path("first-install original", backup)?;
                validate_hash("first-install original", sha256)?;
            }
            for backup in &file.backups {
                validate_backup_path("backup", &backup.path)?;
                validate_hash("backup", &backup.sha256)?;
                if backup.reason.trim().is_empty() {
                    return Err("recovery receipt backup reason is empty".into());
                }
            }
        }
        let probe = AssignmentLedger {
            ledger_version: LEDGER_VERSION,
            assignment_id: self.assignment_id.clone(),
            plugin_id: self.plugin_id.clone(),
            plugin_version: self.plugin_version.clone(),
            revision_sha256: self.revision_sha256.clone(),
            game_id: self.game_id,

            game_folder: PathBuf::from(r"\\?\receipt-validation"),
            selected_target: PathBuf::from(r"\\?\receipt-validation\game.exe"),
            resolved_launcher: None,
            resolved_watch_target: None,
            state: AssignmentState::Active,
            files: Vec::new(),
            retired_files: Vec::new(),
            transactions: Vec::new(),
            install_script_receipts: self.install_script_receipts.clone(),
        };
        probe.validate()?;
        Ok(())
    }

    pub fn validate_assignment_identity(
        &self,
        game_id: u64,
        assignment_id: &str,
        plugin_id: &str,
        plugin_version: &semver::Version,
        revision_sha256: &str,
    ) -> Result<(), String> {
        self.validate()?;
        if self.game_id != game_id
            || self.assignment_id != assignment_id
            || self.plugin_id != plugin_id
            || self.plugin_version != plugin_version.to_string()
            || !self.revision_sha256.eq_ignore_ascii_case(revision_sha256)
        {
            return Err("recovery receipt identity does not match the game assignment".into());
        }
        Ok(())
    }
}

fn validate_backup_path(label: &str, path: &std::path::Path) -> Result<(), String> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(format!(
            "recovery receipt {label} path is not absolute: {}",
            path.display()
        ))
    }
}

fn validate_hash(label: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "{label} SHA-256 must contain exactly 64 hexadecimal characters"
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModificationDecision {
    BackupAndOverwrite,
    KeepCurrent,
    RestoreOriginal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModificationPurpose {
    ReplaceWithPlugin,
    RestoreOriginal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserModification {
    pub destination: PathBuf,
    pub purpose: ModificationPurpose,
    pub last_deployed_sha256: String,
    pub current_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailurePoint {
    Never,
    AfterPlanned,
    AfterStaged(usize),
    AfterBackedUp(usize),
    AfterCommitting,
    AfterCommitted(usize),
}
