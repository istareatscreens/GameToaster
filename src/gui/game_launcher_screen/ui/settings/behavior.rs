use eframe::egui;
use rust_i18n::t;

use super::SettingsDraft;
use crate::game_launcher_screen::LauncherXAction;
use crate::gui::common::components::Combo;

pub(super) fn show(ui: &mut egui::Ui, draft: &mut SettingsDraft) {
    ui.heading(t!("controller.behavior.heading").to_string());
    crate::gui::common::components::checkbox(
        ui,
        &mut draft.value.launch_on_start,
        t!("controller.behavior.launch_on_start").to_string(),
    );
    crate::gui::common::components::checkbox(
        ui,
        &mut draft.value.reopen_after_game_close,
        t!("controller.behavior.reopen_after_close").to_string(),
    );
    crate::gui::common::components::checkbox(
        ui,
        &mut draft.value.reopen_on_monitor_return,
        t!("controller.behavior.reopen_on_monitor_return").to_string(),
    );
    ui.add_enabled_ui(draft.value.launch_on_start, |ui| {
        crate::gui::common::components::checkbox(
            ui,
            &mut draft.value.controller_only_start,
            t!("controller.behavior.hide_main").to_string(),
        );
    });
    if !draft.value.launch_on_start {
        draft.value.controller_only_start = false;
    }

    ui.add_enabled_ui(draft.startup_enabled, |ui| {
        crate::gui::common::components::checkbox(
            ui,
            &mut draft.value.open_on_os_startup,
            t!("controller.behavior.open_on_os_startup").to_string(),
        );
    });
    if !draft.startup_enabled {
        ui.label(crate::gui::common::components::text::hint(t!(
            "controller.behavior.open_on_os_startup_hint"
        )));
    }
    ui.horizontal(|ui| {
        crate::gui::common::components::checkbox(
            ui,
            &mut draft.value.ba2_enabled,
            t!("controller.behavior.ba2_toggle").to_string(),
        );
        crate::gui::common::components::checkbox(
            ui,
            &mut draft.value.ba1_enabled,
            t!("controller.behavior.ba1_enable").to_string(),
        );
    });
    ui.add_enabled_ui(draft.value.ba1_enabled, |ui| {
        Combo::labelled(
            t!("controller.action.label"),
            &mut draft.value.launcher_x_action,
        )
        .choice(
            LauncherXAction::MinimizeToTray,
            t!("controller.action.minimize"),
        )
        .choice(LauncherXAction::ExitLauncher, t!("controller.action.exit"))
        .show(ui);
    });

    ui.add_enabled_ui(
        draft.value.ba1_enabled && draft.value.launcher_x_action == LauncherXAction::MinimizeToTray,
        |ui| {
            crate::gui::common::components::checkbox(
                ui,
                &mut draft.value.keep_controller_alive_when_minimized,
                t!("controller.behavior.keep_alive_minimized").to_string(),
            );
        },
    );
}
