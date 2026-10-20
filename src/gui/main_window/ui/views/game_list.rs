use std::collections::BTreeMap;

use eframe::egui::{self, FontId, RichText, Sense, Vec2};
use egui_phosphor::regular as icons;
use rust_i18n::t;

use crate::gui::common::components::{self, text, TileTone};
use crate::gui::common::theme;
use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig};
use crate::plugin::runtime::{GameMode, RuntimeStatus, Stage};

use super::UiAction;

const ROW_HEIGHT: f32 = 68.0;
const ROW_SPACING: f32 = 8.0;

pub(super) fn game_list(
    ui: &mut egui::Ui,
    config: &LauncherConfig,
    plugin_status: &BTreeMap<GameId, RuntimeStatus>,
    disabled: bool,
    height: f32,
    actions: &mut Vec<UiAction>,
) {
    let panel = theme::frames::list_panel();
    let inner_height = (height - panel.inner_margin.sum().y).max(80.0);
    panel.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(inner_height);

        theme::apply_scroll_bar_visuals(ui);

        egui::ScrollArea::vertical()
            .id_salt("launcher-game-list")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .max_height(inner_height)
            .show(ui, |ui| {
                if config.games.is_empty() {
                    empty_state(ui);
                    return;
                }
                for (index, game) in config.games.iter().enumerate() {
                    game_row(
                        ui,
                        game,
                        plugin_status.get(&game.id),
                        config.default_game == Some(game.id),
                        disabled,
                        actions,
                    );
                    if index + 1 < config.games.len() {
                        ui.add_space(ROW_SPACING);
                    }
                }
            });
    });
}

fn empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(
            RichText::new(t!("main.empty_title").to_string())
                .strong()
                .size(theme::font_size::LARGE),
        );
        ui.add_space(6.0);
        ui.label(text::note(t!("main.empty_hint")));
    });
}

fn game_row(
    ui: &mut egui::Ui,
    game: &GameEntry,
    plugin_status: Option<&RuntimeStatus>,
    is_default: bool,
    disabled: bool,
    actions: &mut Vec<UiAction>,
) {
    theme::frames::list_row().show(ui, |ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            let controls_width = components::SWITCH_SIZE.x
                + components::TILE_SIZE.x * 6.0
                + ui.spacing().item_spacing.x * 7.0;
            let title_width = (ui.available_width() - controls_width).max(140.0);

            let sense = if disabled {
                Sense::hover()
            } else {
                Sense::click()
            };
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(title_width, ROW_HEIGHT), sense);
            if response.hovered() && !disabled {
                ui.painter()
                    .rect_filled(rect, 6.0, theme::BANNER_LAVENDER.gamma_multiply(0.5));
            }
            let painter = ui.painter().with_clip_rect(rect);
            let block_top = rect.center().y - 30.0;
            let left = rect.left() + 6.0;
            painter.text(
                egui::pos2(left, block_top),
                egui::Align2::LEFT_TOP,
                &game.label,
                FontId::proportional(theme::font_size::LARGE),
                theme::INK,
            );
            painter.text(
                egui::pos2(left, block_top + 24.0),
                egui::Align2::LEFT_TOP,
                game.selected_target.display().to_string(),
                FontId::proportional(theme::font_size::CAPTION),
                theme::PATH_BLUE,
            );
            if let Some(status) = plugin_status.and_then(runtime_status_text) {
                painter.text(
                    egui::pos2(left, block_top + 46.0),
                    egui::Align2::LEFT_TOP,
                    status,
                    FontId::proportional(theme::font_size::SMALL),
                    theme::MUTED,
                );
            }
            if response.clicked() {
                actions.push(UiAction::Launch(game.id));
            }
            if !disabled {
                response
                    .on_hover_text(t!("tooltip.launch", name => game.label.as_str()).to_string());
            }

            let mut default = is_default;
            let switch = components::switch(ui, &mut default, !disabled);
            let switch = if disabled {
                switch
            } else {
                switch.on_hover_text(
                    if is_default {
                        t!("tooltip.default_on")
                    } else {
                        t!("tooltip.default_off")
                    }
                    .to_string(),
                )
            };
            if switch.changed() {
                actions.push(UiAction::SetDefault(default.then_some(game.id)));
            }
            if action_tile(ui, ActionIcon::Edit, !disabled)
                .on_hover_text(t!("tooltip.edit").to_string())
                .clicked()
            {
                actions.push(UiAction::Rename(game.id));
            }

            let config_enabled = !disabled && !game.config_targets.is_empty();
            let config_response = action_tile(ui, ActionIcon::Config, config_enabled);
            if config_response.clicked() {
                actions.push(UiAction::Config(game.id));
            }
            if config_enabled {
                config_response.on_hover_text(t!("tooltip.config").to_string());
            } else if !disabled {
                config_response.on_hover_text(t!("tooltip.config_disabled").to_string());
            }
            if action_tile(ui, ActionIcon::OpenFolder, !disabled)
                .on_hover_text(t!("tooltip.open_folder").to_string())
                .clicked()
            {
                actions.push(UiAction::OpenFolder(game.id));
            }
            if action_tile(ui, ActionIcon::CloseConfig, !disabled)
                .on_hover_text(t!("tooltip.close_config").to_string())
                .clicked()
            {
                actions.push(UiAction::ConfigureClose(game.id));
            }
            if action_tile(ui, ActionIcon::CloseGame, !disabled)
                .on_hover_text(t!("tooltip.close_game").to_string())
                .clicked()
            {
                actions.push(UiAction::CloseGame(game.id));
            }
            if action_tile(ui, ActionIcon::Remove, !disabled)
                .on_hover_text(t!("tooltip.remove").to_string())
                .clicked()
            {
                actions.push(UiAction::Remove(game.id));
            }
        });
    });
}

fn runtime_status_text(status: &RuntimeStatus) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(progress) = &status.progress {
        let stage = stage_label(&progress.stage);
        parts.push(match progress.percent {
            Some(percent) => t!(
                "plugin_runtime.progress",
                stage => stage,
                percent => format!("{percent}")
            )
            .to_string(),
            None => stage,
        });
        if !progress.text.is_empty() {
            parts.push(progress.text.clone());
        }
    }
    if let Some(state) = status.game_state {
        if state.payment_known() {
            let kind = if state.free_play {
                t!("plugin_runtime.free_play").to_string()
            } else if state.paid_play.unwrap_or(false) {
                t!("plugin_runtime.paid_play").to_string()
            } else {
                t!("plugin_runtime.credit_ready").to_string()
            };
            parts.push(
                t!(
                    "plugin_runtime.coin",
                    state => kind,
                    count => state.credits.unwrap_or(0).to_string()
                )
                .to_string(),
            );
        }
        if state.mode != GameMode::Unknown {
            parts.push(t!("plugin_runtime.game", mode => game_mode_label(state.mode)).to_string());
        }
    }
    (!parts.is_empty()).then(|| parts.join(" • "))
}

fn stage_label(stage: &Stage) -> String {
    match stage {
        Stage::Validating => t!("plugin_runtime.stage.validating"),
        Stage::BackingUp => t!("plugin_runtime.stage.backing_up"),
        Stage::Preparing => t!("plugin_runtime.stage.preparing"),
        Stage::Patching => t!("plugin_runtime.stage.patching"),
        Stage::Verifying => t!("plugin_runtime.stage.verifying"),
        Stage::Launching => t!("plugin_runtime.stage.launching"),
        Stage::Running => t!("plugin_runtime.stage.running"),
        Stage::Restoring => t!("plugin_runtime.stage.restoring"),
        Stage::Complete => t!("plugin_runtime.stage.complete"),
        Stage::Other(word) => return word.clone(),
    }
    .to_string()
}

fn game_mode_label(mode: GameMode) -> String {
    match mode {
        GameMode::Unknown => t!("plugin_runtime.mode.unknown"),
        GameMode::Attract => t!("plugin_runtime.mode.attract"),
        GameMode::Ready => t!("plugin_runtime.mode.ready"),
        GameMode::ActivePlay => t!("plugin_runtime.mode.active_play"),
        GameMode::Results => t!("plugin_runtime.mode.results"),
    }
    .to_string()
}

#[derive(Clone, Copy)]
enum ActionIcon {
    Edit,
    Config,
    OpenFolder,
    CloseConfig,
    CloseGame,
    Remove,
}

fn action_tile(ui: &mut egui::Ui, icon: ActionIcon, enabled: bool) -> egui::Response {
    let (glyph, fill, tone) = match icon {
        ActionIcon::Edit => (icons::PENCIL_SIMPLE, theme::RAINBOW_SKY, TileTone::Plain),
        ActionIcon::Config => (icons::GEAR, theme::RAINBOW_TEAL, TileTone::Plain),
        ActionIcon::OpenFolder => (icons::FOLDER_OPEN, theme::RAINBOW_MINT, TileTone::Plain),
        ActionIcon::CloseConfig => (icons::SLIDERS, theme::RAINBOW_GOLD, TileTone::Plain),
        ActionIcon::CloseGame => (icons::POWER, theme::RAINBOW_PEACH, TileTone::Plain),
        ActionIcon::Remove => (icons::TRASH, theme::RAINBOW_ROSE, TileTone::Danger),
    };
    components::tile_button(ui, glyph, fill, tone, enabled)
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/views/game_list__tests.rs"]
mod tests;
