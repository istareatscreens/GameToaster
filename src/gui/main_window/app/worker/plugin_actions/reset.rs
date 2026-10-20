use rust_i18n::t;

use crate::gui::main_window::app::worker::plugin_backend::{with_plugin_work, PluginBackend};
use crate::launcher_gui::model::GameEntry;

pub(super) fn reset_game_plugin_config(
    game: &GameEntry,
    backend: &Result<PluginBackend, String>,
) -> Result<crate::plugin::deployment::ConfigurationReset, String> {
    with_plugin_work(backend, |backend| {
        let assignment = game
            .plugin
            .as_ref()
            .ok_or_else(|| t!("errors.no_plugin_assignment").to_string())?;
        backend
            .deployment
            .reset_configuration_files(game.id.0, &assignment.assignment_id)
    })
}
