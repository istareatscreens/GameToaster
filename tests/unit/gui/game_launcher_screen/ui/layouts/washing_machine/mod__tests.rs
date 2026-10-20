use std::path::Path;

use image::{Rgba, RgbaImage};

use super::super::textures::CacheKey;
use super::*;
use crate::launcher_gui::model::GameId;

const SURROUND: Color32 = Color32::BLACK;

pub(super) fn save_preview(relative: &str, image: &RgbaImage) {
    let Some(folder) = std::env::var_os("GAMETOASTER_TEST_PREVIEWS") else {
        return;
    };
    let path = std::path::Path::new(&folder).join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    image.save(&path).unwrap();
}

fn point_in_polygon(px: f32, py: f32, poly: &[egui::Pos2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = (poly[i].x, poly[i].y);
        let (xj, yj) = (poly[j].x, poly[j].y);
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[test]
fn preview_selected_orange_row_shape() {
    let (w, h) = (1200u32, 1200u32);
    let center = egui::pos2(600.0, 600.0);
    let radius = 560.0f32;
    let scale =
        (radius * 2.0) / crate::game_launcher_screen::surface::circle::metrics::CIRCLE_UNITS;
    let ring_inner = (radius - (10.0 * scale).max(2.0)).max(0.0);
    let ring_mid = radius - (5.0 * scale).max(1.0);
    let ring_half = (10.0 * scale).max(2.0) / 2.0;

    let half_height = ROW_HEIGHT * scale / 2.0;
    let (top, bottom) = (center.y - half_height, center.y + half_height);
    let selected_width = (SELECTED_MAX_WIDTH * scale).min(radius * 2.0 - 120.0 * scale);
    let left = center.x - selected_width / 2.0;

    let selected_reach = (radius - (SELECTED_ROW_RING_INSET * scale).max(1.0)).max(0.0);
    let corner = (20.0 * scale).clamp(2.0, 40.0).min(half_height);

    let mut poly = Vec::new();
    let (corner_samples, arc_samples) = (6, 40);
    let tl = egui::pos2(left + corner, top + corner);
    for i in 0..=corner_samples {
        let a = std::f32::consts::PI * (1.0 + 0.5 * i as f32 / corner_samples as f32);
        poly.push(tl + Vec2::new(a.cos(), a.sin()) * corner);
    }
    for i in 0..=arc_samples {
        let arc_y = top + (bottom - top) * (i as f32 / arc_samples as f32);
        let dy = arc_y - center.y;
        let arc_x = center.x + (selected_reach * selected_reach - dy * dy).max(0.0).sqrt();
        poly.push(egui::pos2(arc_x.max(left + corner), arc_y));
    }
    let bl = egui::pos2(left + corner, bottom - corner);
    for i in 0..=corner_samples {
        let a = std::f32::consts::PI * (0.5 + 0.5 * i as f32 / corner_samples as f32);
        poly.push(bl + Vec2::new(a.cos(), a.sin()) * corner);
    }

    let orange = Rgba([0xFF, 0xA1, 0x6C, 0xFF]);
    let lavender = Rgba([0xC5, 0xCF, 0xFE, 0xFF]);
    let mint = Rgba([0x8C, 0xE7, 0xE2, 0xFF]);
    let black = Rgba([0, 0, 0, 0xFF]);
    let mut img = RgbaImage::from_pixel(w, h, black);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = ((fx - center.x).powi(2) + (fy - center.y).powi(2)).sqrt();
            let color = if point_in_polygon(fx, fy, &poly) {
                orange
            } else if (d - ring_mid).abs() <= ring_half {
                mint
            } else if d <= radius {
                lavender
            } else {
                black
            };
            img.put_pixel(x, y, color);
        }
    }
    save_preview("orange_bar_preview.png", &img);

    let mid_right = center.x + selected_reach;
    assert!(
        selected_reach > ring_inner,
        "selected row must reach past the ring inner edge"
    );
    assert!(point_in_polygon(mid_right - 1.0, center.y, &poly));
    assert!(!point_in_polygon(mid_right + 2.0, center.y, &poly));

    let corner_reach = (selected_reach * selected_reach - half_height * half_height)
        .max(0.0)
        .sqrt();
    assert!(
        corner_reach >= ring_inner,
        "selected row's top/bottom corners must still reach the ring (no wedge)"
    );
    assert!(!point_in_polygon(left + 1.0, top + 1.0, &poly));
    assert!(point_in_polygon(left + corner + 2.0, center.y, &poly));
}

#[test]
fn deactivation_releases_every_cached_texture() {
    let context = egui::Context::default();
    let mut renderer = WashingMachineLayout::default();
    let key = CacheKey {
        game_id: GameId(1),
        target_bucket: 3,
    };
    renderer.cache.insert_texture(
        key,
        context.load_texture(
            "deactivation-test",
            egui::ColorImage::new([1, 1], vec![Color32::WHITE]),
            egui::TextureOptions::LINEAR,
        ),
    );
    assert_eq!(renderer.resource_count(), 1);
    renderer.deactivate();
    assert_eq!(renderer.resource_count(), 0);
}

#[test]
fn resolution_screenshot_matrix_preserves_circle_mask_rows_and_artwork() {
    use crate::game_launcher_screen::display::geometry::PixelRect;
    use crate::game_launcher_screen::surface::circle::metrics::{
        canonical_scale, safe_chord_width, ARTWORK_UNITS,
    };
    use crate::game_launcher_screen::surface::circle::{circle_rect, fit_only_circle};
    use crate::game_launcher_screen::OrientationProfile;

    let cases = [
        (
            "1080x1920-portrait",
            1080,
            1920,
            OrientationProfile::CabinetPortrait,
            None,
        ),
        (
            "2160x4096-portrait",
            2160,
            4096,
            OrientationProfile::CabinetPortrait,
            None,
        ),
        (
            "1080x1440-production",
            1080,
            1440,
            OrientationProfile::ProductionLandscape,
            None,
        ),
        (
            "1280x1440-production",
            1280,
            1440,
            OrientationProfile::ProductionLandscape,
            None,
        ),
        (
            "1920x1080-production",
            1920,
            1080,
            OrientationProfile::ProductionLandscape,
            None,
        ),
        (
            "4096x2160-production",
            4096,
            2160,
            OrientationProfile::ProductionLandscape,
            None,
        ),
        (
            "1600x1200-arbitrary",
            1600,
            1200,
            OrientationProfile::ProductionLandscape,
            Some(900),
        ),
    ];
    for (name, width, height, profile, calibrated) in cases {
        let monitor = PixelRect {
            x: 0,
            y: 0,
            width,
            height,
        };
        let circle = match calibrated {
            Some(diameter) => circle_rect(monitor, diameter, profile).unwrap(),
            None => fit_only_circle(monitor, profile).unwrap(),
        };
        let mut image = RgbaImage::from_pixel(width, height, Rgba(SURROUND.to_array()));
        let radius = circle.width as f32 / 2.0;
        let center_x = circle.x as f32 + radius;
        let center_y = circle.y as f32 + radius;
        for y in circle.y..circle.y + circle.height as i32 {
            for x in circle.x..circle.x + circle.width as i32 {
                let dx = x as f32 + 0.5 - center_x;
                let dy = y as f32 + 0.5 - center_y;
                if dx * dx + dy * dy <= radius * radius {
                    image.put_pixel(x as u32, y as u32, Rgba(theme::LIST_PANEL.to_array()));
                }
            }
        }

        let scale = canonical_scale(circle.width);
        let row_height = (ROW_HEIGHT * scale).round().max(1.0) as u32;
        let row_width = safe_chord_width(
            circle.width as f32,
            0.0,
            row_height as f32 / 2.0,
            60.0 * scale,
        )
        .min(SELECTED_MAX_WIDTH * scale)
        .round()
        .max(1.0) as u32;
        let row_x = (center_x - row_width as f32 / 2.0).round() as i32;
        let row_y = (center_y - row_height as f32 / 2.0).round() as i32;

        let selected_reach = (radius - (SELECTED_ROW_RING_INSET * scale).max(1.0)).max(0.0);
        for py in row_y..(row_y + row_height as i32) {
            let dy = py as f32 + 0.5 - center_y;
            let arc_x = center_x + (selected_reach * selected_reach - dy * dy).max(0.0).sqrt();
            let width = (arc_x - row_x as f32).round().max(1.0) as u32;
            fill_rect(
                &mut image,
                row_x,
                py,
                width,
                1,
                theme::PRIMARY_ORANGE.to_array(),
            );
        }

        let artwork_side = (ARTWORK_UNITS * scale).round().max(1.0) as u32;
        let artwork_x = row_x + (ROW_CONTENT_INSET * scale).round() as i32;
        let artwork_y = (center_y - artwork_side as f32 / 2.0).round() as i32;
        fill_rect(
            &mut image,
            artwork_x,
            artwork_y,
            artwork_side,
            artwork_side,
            theme::ICON_TILE.to_array(),
        );

        assert_eq!(artwork_side, (96.0 * scale).round() as u32, "{name}");
        assert!(row_width <= circle.width, "{name}");
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel.0 != SURROUND.to_array() {
                let dx = x as f32 + 0.5 - center_x;
                let dy = y as f32 + 0.5 - center_y;
                assert!(
                    dx * dx + dy * dy <= radius * radius + 1.0,
                    "{name} at {x},{y}"
                );
            }
        }
        assert_eq!(image.get_pixel(0, 0).0, SURROUND.to_array(), "{name}");
        save_preview(&format!("resolution_matrix/{name}.png"), &image);
    }
}

#[test]
fn label_starts_at_content_inset_without_artwork_and_after_the_box_with_it() {
    let scale = 1.0;
    let content_left = 130.0;
    let artwork_size = crate::game_launcher_screen::surface::circle::metrics::ARTWORK_UNITS * scale;

    assert_eq!(
        row_label_left(false, content_left, artwork_size, scale),
        content_left
    );

    assert_eq!(
        row_label_left(true, content_left, artwork_size, scale),
        content_left + artwork_size + 24.0 * scale
    );
}

#[test]
fn visible_rows_stay_inside_the_circle_at_every_diameter() {
    use crate::game_launcher_screen::surface::circle::metrics::{safe_chord_width, CIRCLE_UNITS};
    for diameter in [540.0_f32, 777.0, 1080.0, 2160.0] {
        let radius = diameter / 2.0;
        let scale = diameter / CIRCLE_UNITS;
        let center_index = (VISIBLE_ROWS / 2) as f32;
        for index in 0..VISIBLE_ROWS {
            let relative_y = (index as f32 - center_index) * ROW_PITCH * scale;
            let safe =
                safe_chord_width(diameter, relative_y, ROW_HALF_HEIGHT * scale, 60.0 * scale)
                    .max(1.0);
            let unselected_width = safe.min(UNSELECTED_MAX_WIDTH * scale);
            let corner_y = relative_y.abs() + ROW_HEIGHT * scale / 2.0;

            let half_w = unselected_width / 2.0;
            assert!(
                half_w * half_w + corner_y * corner_y <= radius * radius + 1.0,
                "row {index} at diameter {diameter} escapes the circle"
            );

            let selected_reach = (radius - (SELECTED_ROW_RING_INSET * scale).max(1.0)).max(0.0);
            assert!(
                selected_reach < radius,
                "selected reach must be inside the circle"
            );
            let selected_right = (selected_reach * selected_reach - corner_y * corner_y)
                .max(0.0)
                .sqrt();
            assert!(
                selected_right * selected_right + corner_y * corner_y <= radius * radius + 1.0,
                "selected row {index} at diameter {diameter} escapes the circle on the right"
            );
        }
    }
}

#[test]
fn selected_row_border_covers_left_top_bottom_but_not_the_right_arc() {
    let radius = 540.0_f32;
    let scale = 1.0;
    let center = egui::pos2(radius, radius);
    let half_height = ROW_HEIGHT * scale / 2.0;
    let (top, bottom) = (center.y - half_height, center.y + half_height);
    let selected_width = (SELECTED_MAX_WIDTH * scale).min(radius * 2.0 - 120.0 * scale);
    let left = center.x - selected_width / 2.0;
    let selected_reach = (radius - (SELECTED_ROW_RING_INSET * scale).max(1.0)).max(0.0);
    let corner = (20.0 * scale).clamp(2.0, 40.0).min(half_height);
    let corner_samples = 6;
    let tl = egui::pos2(left + corner, top + corner);
    let bl = egui::pos2(left + corner, bottom - corner);
    let arc_x_at = |arc_y: f32| {
        let dy = arc_y - center.y;
        (center.x + (selected_reach * selected_reach - dy * dy).max(0.0).sqrt()).max(left + corner)
    };
    let mut border = Vec::new();
    border.push(egui::pos2(arc_x_at(bottom), bottom));
    for i in 0..=corner_samples {
        let a = std::f32::consts::PI * (0.5 + 0.5 * i as f32 / corner_samples as f32);
        border.push(bl + Vec2::new(a.cos(), a.sin()) * corner);
    }
    for i in 0..=corner_samples {
        let a = std::f32::consts::PI * (1.0 + 0.5 * i as f32 / corner_samples as f32);
        border.push(tl + Vec2::new(a.cos(), a.sin()) * corner);
    }
    border.push(egui::pos2(arc_x_at(top), top));

    let min_x = border.iter().fold(f32::MAX, |m, p| m.min(p.x));
    let max_x = border.iter().fold(f32::MIN, |m, p| m.max(p.x));
    let min_y = border.iter().fold(f32::MAX, |m, p| m.min(p.y));
    let max_y = border.iter().fold(f32::MIN, |m, p| m.max(p.y));

    assert!(
        (min_x - left).abs() < 1.0,
        "border must reach the left edge"
    );
    assert!((min_y - top).abs() < 1.0, "border must reach the top edge");
    assert!(
        (max_y - bottom).abs() < 1.0,
        "border must reach the bottom edge"
    );

    assert!(
        max_x < center.x + selected_reach - 1.0,
        "border must not wrap the right arc"
    );
}

fn fill_rect(image: &mut RgbaImage, x: i32, y: i32, width: u32, height: u32, color: [u8; 4]) {
    for py in y.max(0) as u32..(y + height as i32).min(image.height() as i32).max(0) as u32 {
        for px in x.max(0) as u32..(x + width as i32).min(image.width() as i32).max(0) as u32 {
            image.put_pixel(px, py, Rgba(color));
        }
    }
}

fn textured_meshes(
    context: &egui::Context,
    layout: &mut WashingMachineLayout,
    frame: &LauncherFrame<'_>,
) -> usize {
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        let _ = layout.render(ui, frame, None);
    });
    crate::gui::common::harness::painted_shapes(&output)
        .iter()
        .filter(|shape| match shape {
            egui::Shape::Mesh(mesh) => mesh.texture_id != egui::TextureId::default(),
            _ => false,
        })
        .count()
}

#[test]
fn the_backdrop_draws_the_frames_background_and_the_themed_fill_without_one() {
    use crate::game_launcher_screen::display::geometry::PixelRect;
    use crate::game_launcher_screen::domain::circular_game_list::CircularGameList;
    use crate::launcher_gui::model::LauncherConfig;
    use image::{ImageBuffer, Rgba};
    let dir =
        std::env::temp_dir().join(format!("gametoaster-backdrop-frame-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("launcher_assets/backgrounds")).unwrap();
    ImageBuffer::from_pixel(4, 2, Rgba([9u8, 8, 7, 255]))
        .save(dir.join("launcher_assets/backgrounds/picked.png"))
        .unwrap();
    let context = egui::Context::default();
    let config = LauncherConfig::default();
    let games = CircularGameList::default();
    let mut layout = WashingMachineLayout::default();
    let frame = |background: Option<&'static Path>| LauncherFrame {
        launcher_dir: &dir,
        config: &config,
        background,
        games: &games,
        surface_px: PixelRect {
            x: 0,
            y: 0,
            width: 800,
            height: 800,
        },
        mode: LauncherFrameMode::Menu {
            credit_readout: None,
        },
    };
    assert_eq!(textured_meshes(&context, &mut layout, &frame(None)), 0);
    let picked = Path::new("launcher_assets/backgrounds/picked.png");
    assert!(
        textured_meshes(&context, &mut layout, &frame(Some(picked))) >= 1,
        "the frame's background fills the circle"
    );
    assert_eq!(
        textured_meshes(&context, &mut layout, &frame(None)),
        0,
        "without a background the themed fill returns"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
