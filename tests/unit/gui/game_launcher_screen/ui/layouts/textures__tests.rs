use eframe::egui::Color32;

use super::*;

#[test]
fn texture_cache_is_bounded_and_evicts_the_oldest_bucket() {
    let context = egui::Context::default();
    let mut renderer = LayoutTextures::default();
    for index in 0..=MAX_TEXTURES {
        let key = CacheKey {
            game_id: GameId(index as u64 + 1),
            target_bucket: 3,
        };
        let texture = context.load_texture(
            format!("cache-test-{index}"),
            egui::ColorImage::new([1, 1], vec![Color32::WHITE]),
            egui::TextureOptions::LINEAR,
        );
        renderer.insert_texture(key, texture);
    }
    assert_eq!(renderer.textures.len(), MAX_TEXTURES);
    assert!(!renderer.textures.keys().any(|key| key.game_id == GameId(1)));
    assert!(renderer
        .textures
        .keys()
        .any(|key| key.game_id == GameId(MAX_TEXTURES as u64 + 1)));
}

#[test]
fn a_changed_background_path_reloads_the_texture() {
    use image::{ImageBuffer, Rgba};
    let dir = std::env::temp_dir().join(format!(
        "gametoaster-background-texture-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let backgrounds = dir.join("launcher_assets/backgrounds");
    std::fs::create_dir_all(&backgrounds).unwrap();
    let picture = |name: &str, width: u32| {
        ImageBuffer::from_pixel(width, 1, Rgba([1u8, 2, 3, 255]))
            .save(backgrounds.join(name))
            .unwrap();
        PathBuf::from("launcher_assets/backgrounds").join(name)
    };
    let two = picture("two.png", 2);
    let three = picture("three.png", 3);
    let context = egui::Context::default();
    let mut textures = LayoutTextures::default();
    let size = |textures: &mut LayoutTextures, path: &Path| {
        textures
            .background_texture(&context, &dir, path)
            .map(|texture| texture.size())
    };
    assert_eq!(size(&mut textures, &two), Some([2, 1]));
    assert_eq!(
        size(&mut textures, &three),
        Some([3, 1]),
        "a new path reloads"
    );
    assert_eq!(size(&mut textures, &two), Some([2, 1]));
    assert_eq!(
        size(
            &mut textures,
            Path::new("launcher_assets/backgrounds/missing.png")
        ),
        None,
        "a missing image draws the themed background"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
