use std::fs;

use super::backup::{create_dated_backup, move_new, replace_existing, stage_bytes_adjacent};
use super::recovery::ensure_no_incomplete_transaction;
use super::{
    is_operator_owned, AssignmentState, BackupRecord, ConfigurationReset, DeploymentEngine,
};
use crate::plugin::catalog::inventory::sha256_file;

impl DeploymentEngine {
    pub fn reset_configuration_files(
        &self,
        game_id: u64,
        assignment_id: &str,
    ) -> Result<ConfigurationReset, String> {
        let mut ledger = self.store.load(game_id, assignment_id)?;
        ensure_no_incomplete_transaction(&ledger)?;
        if ledger.state != AssignmentState::Active {
            return Err("only an active plugin assignment can have its configuration reset".into());
        }
        let mut restored = Vec::new();
        let mut backups = Vec::new();
        for index in 0..ledger.files.len() {
            if !is_operator_owned(ledger.files[index].kind) {
                continue;
            }
            let source = ledger.files[index].source.clone();
            let destination = ledger.files[index].destination.clone();
            let expected = sha256_file(&source).map_err(|error| {
                format!(
                    "the plugin default configuration is unavailable: {}: {error}",
                    source.display()
                )
            })?;
            let bytes = fs::read(&source)
                .map_err(|error| format!("failed to read {}: {error}", source.display()))?;
            let staged = stage_bytes_adjacent(&destination, &bytes, "configuration-reset")?;
            if destination.is_file() {
                let (backup, hash) = create_dated_backup(&destination)?;
                ledger.files[index].backup_chain.push(BackupRecord {
                    path: backup.clone(),
                    sha256: hash,
                    reason: "configuration-reset".into(),
                });
                backups.push(backup);
                replace_existing(&destination, &staged)?;
            } else {
                move_new(&staged, &destination)?;
            }
            let committed = sha256_file(&destination)?;
            if committed != expected {
                return Err(format!(
                    "configuration reset verification failed: {}",
                    destination.display()
                ));
            }
            ledger.files[index].last_deployed_sha256 = committed;
            ledger.files[index].user_kept_current = false;
            restored.push(destination);
        }
        if restored.is_empty() {
            return Err("this plugin declares no configuration files".into());
        }
        self.store.save(&ledger)?;
        Ok(ConfigurationReset { restored, backups })
    }
}
