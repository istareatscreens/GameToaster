use image::{Rgba, RgbaImage};

use super::super::tests::save_preview;
use super::*;

#[test]
fn preview_circle_background_fill() {
    let (iw, ih) = (400u32, 200u32);
    let sample = |u: f32, v: f32| -> Rgba<u8> {
        let (u, v) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
        let cx = if u < 0.5 { 0 } else { 255 };
        let cy = if v < 0.5 { 0 } else { 180 };

        let checker = (((u * 8.0) as u32 + (v * 8.0) as u32) % 2) as u8 * 40;
        Rgba([
            (cx as u8).saturating_add(checker),
            (cy as u8).saturating_add(checker),
            255u8.saturating_sub(checker),
            255,
        ])
    };
    let _ = (iw, ih);

    let (w, h) = (1200u32, 1200u32);
    let center = egui::pos2(600.0, 600.0);
    let radius = 560.0f32;
    let uv = cover_uv(Vec2::splat(radius * 2.0), Vec2::new(iw as f32, ih as f32));
    let black = Rgba([0, 0, 0, 0xFF]);
    let mut img = RgbaImage::from_pixel(w, h, black);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = ((fx - center.x).powi(2) + (fy - center.y).powi(2)).sqrt();
            if d <= radius {
                let bx = (fx - (center.x - radius)) / (radius * 2.0);
                let by = (fy - (center.y - radius)) / (radius * 2.0);
                let u = uv.left() + bx * uv.width();
                let v = uv.top() + by * uv.height();
                img.put_pixel(x, y, sample(u, v));
            }
        }
    }
    save_preview("circle_background_preview.png", &img);

    assert!((uv.left() - 0.25).abs() < 1e-6 && (uv.right() - 0.75).abs() < 1e-6);
    assert!((uv.top()).abs() < 1e-6 && (uv.bottom() - 1.0).abs() < 1e-6);
}

#[test]
fn cover_uv_crops_the_overflowing_axis_and_centers_it() {
    let wide = cover_uv(Vec2::new(100.0, 100.0), Vec2::new(200.0, 100.0));
    assert!((wide.min.y - 0.0).abs() < 1e-6 && (wide.max.y - 1.0).abs() < 1e-6);
    assert!((wide.min.x - 0.25).abs() < 1e-6 && (wide.max.x - 0.75).abs() < 1e-6);

    let tall = cover_uv(Vec2::new(100.0, 100.0), Vec2::new(100.0, 200.0));
    assert!((tall.min.x - 0.0).abs() < 1e-6 && (tall.max.x - 1.0).abs() < 1e-6);
    assert!((tall.min.y - 0.25).abs() < 1e-6 && (tall.max.y - 0.75).abs() < 1e-6);

    let exact = cover_uv(Vec2::new(160.0, 90.0), Vec2::new(1920.0, 1080.0));
    assert!((exact.min.x).abs() < 1e-6 && (exact.max.x - 1.0).abs() < 1e-6);
    assert!((exact.min.y).abs() < 1e-6 && (exact.max.y - 1.0).abs() < 1e-6);

    let degenerate = cover_uv(Vec2::new(0.0, 100.0), Vec2::new(100.0, 100.0));
    assert_eq!(degenerate.min, egui::pos2(0.0, 0.0));
    assert_eq!(degenerate.max, egui::pos2(1.0, 1.0));
}
