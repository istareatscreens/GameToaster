use eframe::egui;

use super::{error_text, scroll_area};
use crate::gui::common::theme::{self, font_size, frames};

const BODY_WIDTH: f32 = 490.0;

const BODY_MAX_HEIGHT: f32 = 624.0;

const ERROR_MAX_HEIGHT: f32 = 156.0;

const FOOTER_SPACE: f32 = 8.0;

pub trait DialogContent {
    fn body(&mut self, ui: &mut egui::Ui);

    fn error(&self) -> Option<&str> {
        None
    }

    fn actions(&mut self, ui: &mut egui::Ui);
}

pub struct ModalDialog {
    title: String,
    id: egui::Id,
}

impl ModalDialog {
    pub fn new(title: impl Into<String>, id: egui::Id) -> Self {
        Self {
            title: title.into(),
            id,
        }
    }

    pub fn show(self, context: &egui::Context, content: &mut dyn DialogContent) {
        modal_window(self.title, self.id).show(context, |ui| {
            theme::apply_widget_visuals(ui);

            ui.set_max_width(BODY_WIDTH + ui.spacing().scroll.allocated_width());
            let top = ui.cursor().top();
            if let Some(error) = content.error() {
                scroll_area(
                    ui,
                    egui::ScrollArea::vertical()
                        .id_salt("dialog-error")
                        .max_height(ERROR_MAX_HEIGHT)
                        .auto_shrink([false, true]),
                    |ui| {
                        ui.set_width(BODY_WIDTH);
                        error_text(ui, error);
                    },
                );
            }
            let error_height = ui.cursor().top() - top;
            scroll_area(
                ui,
                egui::ScrollArea::vertical()
                    .max_height(BODY_MAX_HEIGHT - error_height)
                    .auto_shrink([false, true]),
                |ui| {
                    ui.set_width(BODY_WIDTH);
                    content.body(ui);
                },
            );
            ui.add_space(FOOTER_SPACE);
            ui.set_width(BODY_WIDTH);
            ui.separator();
            ui.horizontal(|ui| content.actions(ui));
        });
    }
}

pub fn focus_once(response: &egui::Response, focused: &mut bool) {
    if !*focused {
        response.request_focus();
        *focused = true;
    }
}

fn modal_window(title: impl Into<String>, id: egui::Id) -> egui::Window<'static> {
    egui::Window::new(egui::RichText::new(title).strong().size(font_size::TITLE))
        .id(id)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .movable(false)
        .auto_sized()
        .frame(frames::modal())
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/common/components/dialog__tests.rs"]
mod tests;
