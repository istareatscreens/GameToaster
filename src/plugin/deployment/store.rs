use std::fs;
use std::path::{Path, PathBuf};

use crate::support::fs::atomic_write;

use super::model::{AssignmentLedger, RecoveryReceipt};

#[derive(Clone, Debug)]
pub struct LedgerStore {
    state_root: PathBuf,
}

impl LedgerStore {
    pub fn new(state_root: impl Into<PathBuf>) -> Result<Self, String> {
        let state_root = state_root.into();
        fs::create_dir_all(&state_root)
            .map_err(|error| format!("failed to create {}: {error}", state_root.display()))?;
        Ok(Self { state_root })
    }

    pub fn assignment_root(&self, game_id: u64, assignment_id: &str) -> Result<PathBuf, String> {
        validate_assignment_id(assignment_id)?;
        Ok(self
            .state_root
            .join("games")
            .join(game_id.to_string())
            .join(assignment_id))
    }

    pub fn ledger_path(&self, game_id: u64, assignment_id: &str) -> Result<PathBuf, String> {
        Ok(self
            .assignment_root(game_id, assignment_id)?
            .join("ledger.toml"))
    }

    pub fn save(&self, ledger: &AssignmentLedger) -> Result<(), String> {
        ledger.validate()?;
        let encoded = toml::to_string_pretty(ledger)
            .map_err(|error| format!("ledger encode failed: {error}"))?;
        atomic_write(
            &self.ledger_path(ledger.game_id, &ledger.assignment_id)?,
            encoded.as_bytes(),
        )?;
        self.save_receipt(ledger)
    }

    pub fn load(&self, game_id: u64, assignment_id: &str) -> Result<AssignmentLedger, String> {
        let path = self.ledger_path(game_id, assignment_id)?;
        let raw = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let ledger: AssignmentLedger = toml::from_str(&raw)
            .map_err(|error| format!("invalid assignment ledger {}: {error}", path.display()))?;
        ledger.validate()?;
        Ok(ledger)
    }

    pub fn load_if_present(
        &self,
        game_id: u64,
        assignment_id: &str,
    ) -> Result<Option<AssignmentLedger>, String> {
        let path = self.ledger_path(game_id, assignment_id)?;
        if !path.exists() {
            return Ok(None);
        }
        self.load(game_id, assignment_id).map(Some)
    }

    pub fn list(&self) -> Result<Vec<AssignmentLedger>, String> {
        let games = self.state_root.join("games");
        if !games.is_dir() {
            return Ok(Vec::new());
        }
        let mut ledgers = Vec::new();
        for game in directories(&games)? {
            let game_id = game
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<u64>().ok())
                .ok_or_else(|| format!("invalid game state directory: {}", game.display()))?;
            for assignment in directories(&game)? {
                let assignment_id = assignment
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        format!(
                            "invalid assignment state directory: {}",
                            assignment.display()
                        )
                    })?;
                ledgers.push(self.load(game_id, assignment_id)?);
            }
        }
        ledgers.sort_by(|left, right| {
            left.game_id
                .cmp(&right.game_id)
                .then_with(|| left.assignment_id.cmp(&right.assignment_id))
        });
        Ok(ledgers)
    }

    pub fn load_receipt(path: &Path) -> Result<RecoveryReceipt, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let receipt: RecoveryReceipt = toml::from_str(&raw)
            .map_err(|error| format!("invalid recovery receipt {}: {error}", path.display()))?;
        receipt.validate()?;
        Ok(receipt)
    }

    fn save_receipt(&self, ledger: &AssignmentLedger) -> Result<(), String> {
        let receipt = RecoveryReceipt::from(ledger);
        receipt.validate()?;
        let encoded = toml::to_string_pretty(&receipt)
            .map_err(|error| format!("recovery receipt encode failed: {error}"))?;
        let path = ledger
            .game_folder
            .join(".GameToaster")
            .join(&ledger.assignment_id)
            .join("receipt.toml");
        atomic_write(&path, encoded.as_bytes())
    }
}

fn directories(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut output = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?
    {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if entry.file_type().is_ok_and(|value| value.is_dir()) {
            output.push(entry.path());
        }
    }
    output.sort();
    Ok(output)
}

pub fn validate_assignment_id(value: &str) -> Result<(), String> {
    if value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("assignment ID must be 128-bit hexadecimal".into())
    }
}
