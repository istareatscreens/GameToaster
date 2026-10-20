use std::path::PathBuf;

use rust_i18n::t;

use crate::gui::main_window::app::worker::plugin_backend::{with_plugin_work, PluginBackend};
use crate::launcher_gui::model::{GameEntry, PluginAssignment};
use crate::plugin::deployment::AssignmentState;

pub(super) fn inspect_game_plugin(
    game: &GameEntry,
    backend: &Result<PluginBackend, String>,
) -> Result<crate::plugin::workflow::AssignmentInspection, String> {
    with_plugin_work(backend, |backend| {
        let assignment = game
            .plugin
            .as_ref()
            .ok_or_else(|| t!("errors.no_plugin_assignment").to_string())?;
        let receipt_path = game
            .game_dir()?
            .join(".GameToaster")
            .join(&assignment.assignment_id)
            .join("receipt.toml");
        let ledger = match backend
            .deployment
            .store()
            .load(game.id.0, &assignment.assignment_id)
        {
            Ok(ledger) => ledger,
            Err(ledger_error) => {
                let receipt = crate::plugin::deployment::LedgerStore::load_receipt(&receipt_path)
                    .map_err(|receipt_error| {
                    t!(
                        "plugins.error.ledger_unavailable",
                        ledger => ledger_error,
                        receipt => receipt_error
                    )
                    .to_string()
                })?;
                return receipt_only_inspection(game, assignment, receipt_path, receipt);
            }
        };
        let convert = |file: &crate::plugin::deployment::FileProvenance| {
            crate::plugin::workflow::ManagedFileInspection {
                kind: file.kind,
                destination: file.destination.clone(),
                first_install_original: file.first_install_original.clone(),
                backup_chain: file.backup_chain.clone(),
            }
        };
        Ok(crate::plugin::workflow::AssignmentInspection {
            assignment_id: ledger.assignment_id.clone(),
            state: ledger.state,
            latest_transaction_state: ledger.transactions.last().map(|value| value.state),
            transaction_count: ledger.transactions.len(),
            receipt_path,
            receipt_only: false,
            active_files: ledger.files.iter().map(convert).collect(),
            retired_files: ledger.retired_files.iter().map(convert).collect(),
        })
    })
}

fn receipt_only_inspection(
    game: &GameEntry,
    assignment: &PluginAssignment,
    receipt_path: PathBuf,
    receipt: crate::plugin::deployment::RecoveryReceipt,
) -> Result<crate::plugin::workflow::AssignmentInspection, String> {
    receipt.validate_assignment_identity(
        game.id.0,
        &assignment.assignment_id,
        &assignment.plugin_id,
        &assignment.revision.version,
        &assignment.revision.sha256,
    )?;
    let files = receipt
        .files
        .into_iter()
        .map(|file| crate::plugin::workflow::ManagedFileInspection {
            kind: file.kind,
            destination: file.destination,
            first_install_original: file.first_install_original,
            backup_chain: file.backups,
        })
        .collect();
    Ok(crate::plugin::workflow::AssignmentInspection {
        assignment_id: receipt.assignment_id,
        state: AssignmentState::RecoveryRequired,
        latest_transaction_state: receipt.transaction_state,
        transaction_count: 0,
        receipt_path,
        receipt_only: true,
        active_files: files,
        retired_files: Vec::new(),
    })
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/app/worker/plugin_actions/inspection__tests.rs"]
mod tests;
