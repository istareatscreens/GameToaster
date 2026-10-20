use rust_i18n::t;

use crate::plugin::catalog::library::PluginDescriptor;

pub(super) fn plugin_choice_label(plugin: &PluginDescriptor) -> String {
    format!(
        "{} {} · {}",
        plugin.manifest.label,
        plugin.revision.plugin_version,
        plugin.digest_summary()
    )
}

pub(super) fn assignment_state_label(state: crate::plugin::deployment::AssignmentState) -> String {
    use crate::plugin::deployment::AssignmentState;
    match state {
        AssignmentState::Active => t!("plugins.state.active").to_string(),
        AssignmentState::Removed => t!("plugins.state.removed").to_string(),
        AssignmentState::RecoveryRequired => t!("plugins.state.recovery_required").to_string(),
    }
}

pub(super) fn transaction_state_label(
    state: crate::plugin::deployment::TransactionState,
) -> String {
    use crate::plugin::deployment::TransactionState;
    match state {
        TransactionState::Planned => t!("plugins.transaction.planned").to_string(),
        TransactionState::Staged => t!("plugins.transaction.staged").to_string(),
        TransactionState::BackedUp => t!("plugins.transaction.backed_up").to_string(),
        TransactionState::Committing => t!("plugins.transaction.committing").to_string(),
        TransactionState::Committed => t!("plugins.transaction.committed").to_string(),
        TransactionState::RollingBack => t!("plugins.transaction.rolling_back").to_string(),
        TransactionState::RolledBack => t!("plugins.transaction.rolled_back").to_string(),
        TransactionState::RecoveryRequired => {
            t!("plugins.transaction.recovery_required").to_string()
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/plugin_labels__tests.rs"]
mod tests;
