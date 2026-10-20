use eframe::egui::{self, CornerRadius, Stroke};

use super::palette::{
    BANNER_LAVENDER, BORDER, CANVAS, ICON_TILE, INK, LIST_PANEL, PATH_BLUE, PRIMARY_ORANGE,
    ROW_CREAM, SCROLL_HANDLE, SELECTED_CYAN,
};

pub(super) fn launcher() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = CANVAS;
    visuals.window_fill = ROW_CREAM;
    visuals.extreme_bg_color = ICON_TILE;
    visuals.faint_bg_color = LIST_PANEL;
    visuals.override_text_color = Some(INK);
    visuals.selection.bg_fill = SELECTED_CYAN;
    visuals.selection.stroke = Stroke::new(1.0_f32, INK);
    visuals.window_stroke = BORDER;
    visuals.window_corner_radius = CornerRadius::same(12);
    visuals.widgets.noninteractive.bg_fill = ROW_CREAM;
    visuals.widgets.noninteractive.bg_stroke = BORDER;
    visuals.widgets.inactive.bg_fill = PRIMARY_ORANGE;
    visuals.widgets.inactive.bg_stroke = BORDER;
    visuals.widgets.inactive.expansion = 0.0;
    visuals.widgets.hovered.bg_fill = PRIMARY_ORANGE.gamma_multiply(1.08);
    visuals.widgets.hovered.bg_stroke = BORDER;
    visuals.widgets.hovered.expansion = 0.0;
    visuals.widgets.active.bg_fill = SELECTED_CYAN;
    visuals.widgets.active.bg_stroke = BORDER;
    visuals.widgets.active.expansion = 0.0;
    visuals
}

pub(super) fn apply_spacing(spacing: &mut egui::style::Spacing) {
    spacing.item_spacing = egui::vec2(10.0, 9.0);
    spacing.button_padding = egui::vec2(14.0, 8.0);
    spacing.window_margin = egui::Margin::same(18);
    let mut scroll = egui::style::ScrollStyle::solid();
    scroll.bar_width = 12.0;
    scroll.handle_min_length = 42.0;
    scroll.bar_inner_margin = 5.0;
    scroll.bar_outer_margin = 2.0;
    spacing.scroll = scroll;
}

pub fn apply_scroll_bar_visuals(ui: &mut egui::Ui) {
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.bg_fill = SCROLL_HANDLE;
    widgets.hovered.bg_fill = SCROLL_HANDLE.gamma_multiply(1.1);
    widgets.active.bg_fill = PATH_BLUE;
}

pub fn apply_widget_visuals(ui: &mut egui::Ui) {
    let ink = Stroke::new(1.0_f32, INK);
    let radius = CornerRadius::same(6);
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(INK);
    visuals.extreme_bg_color = ICON_TILE;
    visuals.selection.bg_fill = SELECTED_CYAN;
    visuals.selection.stroke = ink;

    let widgets = &mut visuals.widgets;
    for (widget, fill) in [
        (&mut widgets.noninteractive, ROW_CREAM),
        (&mut widgets.inactive, ICON_TILE),
        (&mut widgets.hovered, BANNER_LAVENDER),
        (&mut widgets.active, PRIMARY_ORANGE),
        (&mut widgets.open, BANNER_LAVENDER),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = BORDER;
        widget.fg_stroke = ink;
        widget.corner_radius = radius;
    }
}

pub fn with_widget_visuals<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let saved = ui.visuals().clone();
    apply_widget_visuals(ui);
    let added = add(ui);
    *ui.visuals_mut() = saved;
    added
}
