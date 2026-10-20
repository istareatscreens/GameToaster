use eframe::egui::{self, RichText, Vec2};
use rust_i18n::t;

use crate::gui::common::components::{self, text, ButtonRole, DialogContent};
use crate::gui::common::theme;
use crate::launcher_gui::model::GameEntry;

use super::{DialogEvent, PresentationDraft};

pub(super) struct RemoveDialog<'a> {
    game: &'a GameEntry,
    event: &'a mut DialogEvent,
}

impl<'a> RemoveDialog<'a> {
    pub(super) fn new(game: &'a GameEntry, event: &'a mut DialogEvent) -> Self {
        Self { game, event }
    }
}

impl DialogContent for RemoveDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let game = self.game;
        ui.label(t!("remove.confirm", label => game.label.as_str()).to_string());
        ui.label(text::path(game.selected_target.display().to_string()));
        ui.add_space(8.0);
        ui.label(t!("remove.record_only").to_string());
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("common.cancel")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
        if components::action(ui, t!("common.remove")).clicked() {
            *self.event = DialogEvent::RemoveGame(self.game.clone());
        }
    }
}

pub(super) struct PresentationDialog<'a> {
    draft: &'a mut PresentationDraft,
    event: &'a mut DialogEvent,

    submit: bool,
}

impl<'a> PresentationDialog<'a> {
    pub(super) fn new(draft: &'a mut PresentationDraft, event: &'a mut DialogEvent) -> Self {
        Self {
            draft,
            event,
            submit: false,
        }
    }
}

impl DialogContent for PresentationDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let draft = &mut *self.draft;
        let event = &mut *self.event;
        ui.label(RichText::new(t!("add.game_label").to_string()).strong());
        let editor = ui.add(
            egui::TextEdit::singleline(&mut draft.label)
                .desired_width(f32::INFINITY)
                .hint_text(t!("add.label_hint").to_string()),
        );
        components::focus_once(&editor, &mut draft.label_focused);
        ui.label(text::path(draft.game.selected_target.display().to_string()));
        ui.add_space(12.0);
        ui.label(RichText::new(t!("controller.background.artwork_heading").to_string()).strong());
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 8.0, theme::ROW_CREAM);
            ui.painter()
                .rect_stroke(rect, 8.0, theme::BORDER, egui::StrokeKind::Inside);
            if let Some(texture) = &draft.preview {
                ui.painter().image(
                    texture.id(),
                    rect.shrink(4.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    t!("controller.background.no_image").to_string(),
                    egui::FontId::proportional(theme::font_size::SMALL),
                    theme::MUTED,
                );
            }
            ui.vertical(|ui| {
                let image_button = if draft.preview.is_some() {
                    t!("controller.background.replace_image").to_string()
                } else {
                    t!("controller.background.choose_image").to_string()
                };
                if components::action(ui, &image_button).clicked() {
                    *event = DialogEvent::ChooseControllerImage;
                }
                if ui
                    .add_enabled(
                        draft.preview.is_some() || draft.game.controller_image.is_some(),
                        components::button(
                            t!("controller.background.clear_image").to_string(),
                            ButtonRole::Secondary,
                        ),
                    )
                    .clicked()
                {
                    draft.clear_image();
                }
                ui.label(text::hint(t!("controller.background.square_hint")));
            });
        });
        self.submit = self.draft.submission().is_ok()
            && editor.lost_focus()
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
    }

    fn error(&self) -> Option<&str> {
        self.draft.error.as_deref()
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let submission = self.draft.submission();
        let valid = submission.is_ok();
        if components::action(ui, t!("common.cancel")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
        let save = components::action_enabled(ui, valid, t!("common.save")).clicked();
        if save || self.submit {
            let (label, source, clear_image) =
                submission.expect("enabled presentation save must have a valid submission");
            *self.event = DialogEvent::SavePresentation {
                id: self.draft.game.id,
                label,
                source,
                clear_image,
            };
        }
    }
}

pub(super) struct ConfigMenuDialog<'a> {
    game: &'a GameEntry,
    event: &'a mut DialogEvent,
}

impl<'a> ConfigMenuDialog<'a> {
    pub(super) fn new(game: &'a GameEntry, event: &'a mut DialogEvent) -> Self {
        Self { game, event }
    }
}

impl DialogContent for ConfigMenuDialog<'_> {
    fn body(&mut self, ui: &mut egui::Ui) {
        let game = self.game;
        let event = &mut *self.event;
        ui.label(
            RichText::new(t!("cfgmenu.open_for", label => game.label.as_str()).to_string())
                .strong(),
        );
        ui.label(text::hint(t!("cfgmenu.hint")));
        ui.add_space(8.0);
        for target in &game.config_targets {
            let name = target
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("?");
            if ui
                .add(
                    components::button(name, ButtonRole::Secondary)
                        .min_size(Vec2::new(ui.available_width(), 30.0)),
                )
                .on_hover_text(target.display().to_string())
                .clicked()
            {
                *event = DialogEvent::OpenConfigTarget(target.clone());
            }
            ui.add_space(6.0);
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        if components::action(ui, t!("common.cancel")).clicked() {
            *self.event = DialogEvent::Cancel;
        }
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit/gui/main_window/ui/dialogs/lifecycle__tests.rs"]
mod tests;
