use super::*;
use crate::gui::common::harness::{painted_scroll_bars, settled};

#[test]
fn a_scroll_area_on_widget_visuals_shows_its_handle_and_keeps_the_body_colours() {
    let rows = |ui: &mut egui::Ui| {
        for row in 0..40 {
            ui.label(format!("row {row}"));
        }
    };

    let context = egui::Context::default();
    theme::apply(&context);
    let output = settled(&context, None, |ui| {
        theme::apply_widget_visuals(ui);
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, rows);
    });
    let bare = painted_scroll_bars(&output, &context.global_style().spacing.scroll);
    assert_eq!(bare.len(), 2, "{bare:?}");
    assert_eq!(
        bare[0].1, bare[1].1,
        "a bare handle is painted in its track's colour"
    );

    let context = egui::Context::default();
    theme::apply(&context);
    let mut colours = None;
    let output = settled(&context, None, |ui| {
        theme::apply_widget_visuals(ui);
        let host = ui.visuals().widgets.clone();
        let mut body = None;
        scroll_area(ui, egui::ScrollArea::vertical().max_height(120.0), |ui| {
            body = Some(ui.visuals().widgets.clone());
            rows(ui);
        });
        colours = Some((host, body, ui.visuals().widgets.clone()));
    });
    let (host, body, after) = colours.unwrap();
    assert_eq!(
        body.as_ref(),
        Some(&host),
        "the body keeps the host's widget colours"
    );
    assert_eq!(after, host, "the host's widget colours are restored");
    let bars = painted_scroll_bars(&output, &context.global_style().spacing.scroll);
    assert_eq!(bars.len(), 2, "{bars:?}");
    let ((track_rect, track), (handle_rect, handle)) = (bars[0], bars[1]);
    assert_eq!((track, handle), (theme::ICON_TILE, theme::SCROLL_HANDLE));
    assert!(handle_rect.height() < track_rect.height());
}
