use eframe::egui::{self, Align2, FontId, RichText, Sense, Vec2};
use rust_i18n::t;

use crate::gui::common::components::{self, text, ButtonRole, DialogContent};
use crate::gui::common::theme;
use crate::launcher_gui::hotkeys::Hotkey;
use crate::launcher_gui::model::GameId;

use super::drafts::{HotkeyDraft, HotkeyTarget};
use super::DialogEvent;

const FIELD_WIDTH: f32 = 140.0;

pub(super) struct HotkeysDialog<'a> {
    draft: &'a mut HotkeyDraft,
    event: &'a mut DialogEvent,
}

impl<'a> HotkeysDialog<'a> {
    pub(super) fn new(draft: &'a mut HotkeyDraft, event: &'a mut DialogEvent) -> Self {
        Self { draft, event }
    }
}

impl DialogContent for HotkeysDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        if let Some(target) = draft.capturing {
            enum Captured {
                Cancel,
                Set(Hotkey),
            }
            let captured = ui.input(|input| {
                input.events.iter().find_map(|ev| match ev {
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } => Some(Captured::Cancel),
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } => Hotkey::from_egui(*modifiers, *key).map(Captured::Set),
                    _ => None,
                })
            });
            match captured {
                Some(Captured::Cancel) => {
                    draft.capturing = None;
                    draft.error = None;
                }
                Some(Captured::Set(hotkey)) => {
                    if !hotkey.is_valid() {
                        draft.error = Some(t!("hotkeys.err_modifier").to_string());
                    } else if let Some(name) = draft.conflict(hotkey, target) {
                        draft.error =
                            Some(t!("hotkeys.err_conflict", name => name.as_str()).to_string());
                    } else {
                        draft.set(target, Some(hotkey));
                        draft.capturing = None;
                        draft.error = None;
                    }
                }
                None => {}
            }

            ui.ctx().request_repaint();
        }

        ui.label(text::note(t!("hotkeys.instruction")));
        ui.add_space(10.0);

        components::section(ui, |ui| {
            ui.label(RichText::new(t!("hotkeys.open_launcher").to_string()).strong());
            ui.add_space(4.0);
            capture_field(ui, draft, HotkeyTarget::Launcher);

            ui.add_space(8.0);
            ui.label(RichText::new(t!("hotkeys.close_game").to_string()).strong());
            ui.add_space(4.0);
            capture_field(ui, draft, HotkeyTarget::CloseLauncher);
        });

        if !draft.games.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new(t!("hotkeys.per_game").to_string()).strong());
            ui.add_space(4.0);

            let ids: Vec<GameId> = draft.games.iter().map(|row| row.id).collect();
            for id in ids {
                game_hotkey_block(ui, draft, id);
                ui.add_space(8.0);
            }
        }
    }

    fn error(&self) -> Option<&str> {
        self.draft.error.as_deref()
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let draft = &*self.draft;
        let event = &mut *self.event;
        if components::action(ui, t!("common.cancel")).clicked() {
            *event = DialogEvent::Cancel;
        }
        if components::action(ui, t!("common.save").to_string()).clicked() {
            *event = DialogEvent::SaveHotkeys {
                launcher: draft.launcher,
                launcher_close: draft.launcher_close,
                games: draft
                    .games
                    .iter()
                    .map(|row| (row.id, row.launch, row.close))
                    .collect(),
            };
        }
    }
}

fn game_hotkey_block(ui: &mut egui::Ui, draft: &mut HotkeyDraft, id: GameId) {
    let label = draft
        .games
        .iter()
        .find(|row| row.id == id)
        .map(|row| row.label.clone())
        .unwrap_or_default();
    components::section(ui, |ui| {
        ui.label(RichText::new(label).strong());
        ui.add_space(6.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;

            let column = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            ui.vertical(|ui| {
                ui.set_max_width(column);
                ui.label(text::hint(t!("hotkeys.launch")));
                capture_field(ui, draft, HotkeyTarget::GameLaunch(id));
            });
            ui.vertical(|ui| {
                ui.set_max_width(column);
                ui.label(text::hint(t!("hotkeys.close")));
                capture_field(ui, draft, HotkeyTarget::GameClose(id));
            });
        });
    });
}

fn capture_field(ui: &mut egui::Ui, draft: &mut HotkeyDraft, target: HotkeyTarget) {
    let current = draft.binding(target);
    let listening = draft.capturing == Some(target);
    let clear = || {
        components::button(t!("hotkeys.clear").to_string(), ButtonRole::Quiet)
            .min_size(Vec2::new(48.0, 28.0))
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        let clear_width = components::measure(ui, |ui| ui.add(clear())).x;
        let field_w =
            FIELD_WIDTH.min(ui.available_width() - ui.spacing().item_spacing.x - clear_width);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(field_w, 28.0), Sense::click());
        let fill = if listening {
            theme::SELECTED_CYAN
        } else if response.hovered() {
            theme::BANNER_LAVENDER
        } else {
            theme::ICON_TILE
        };
        ui.painter().rect_filled(rect, 6.0, fill);
        ui.painter()
            .rect_stroke(rect, 6.0, theme::BORDER, egui::StrokeKind::Inside);
        let (text, color) = if listening {
            (t!("hotkeys.press_keys").to_string(), theme::INK)
        } else if let Some(hotkey) = current {
            (hotkey.to_string(), theme::INK)
        } else {
            (t!("hotkeys.not_set").to_string(), theme::MUTED)
        };
        ui.painter().with_clip_rect(rect.shrink(6.0)).text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(theme::font_size::CAPTION),
            color,
        );
        if response.clicked() {
            draft.capturing = if listening { None } else { Some(target) };
            draft.error = None;
        }
        if ui.add_enabled(current.is_some(), clear()).clicked() {
            draft.set(target, None);
            draft.error = None;
            if listening {
                draft.capturing = None;
            }
        }
    });
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/hotkeys__tests.rs"]
mod tests;
