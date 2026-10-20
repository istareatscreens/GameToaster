use super::*;

#[test]
fn contain_preserves_aspect_and_stays_within_the_area() {
    let area = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(180.0, 120.0));

    let within = |fit: egui::Rect| {
        let eps = 1e-2;
        fit.center() == area.center()
            && fit.width() <= area.width() + eps
            && fit.height() <= area.height() + eps
    };

    let fit = contain(area, egui::vec2(400.0, 200.0));
    assert!((fit.width() - 180.0).abs() < 1e-2);
    assert!((fit.height() - 90.0).abs() < 1e-2);
    assert!(within(fit));

    let tall = contain(area, egui::vec2(100.0, 200.0));
    assert!((tall.height() - 120.0).abs() < 1e-2);
    assert!((tall.width() - 60.0).abs() < 1e-2);
    assert!(within(tall));

    assert_eq!(contain(area, egui::vec2(0.0, 10.0)), area);
}
