use super::*;
use image::{GenericImageView, ImageBuffer, Rgba};

fn test_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "gametoaster-controller-artwork-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn png_is_contained_in_square_and_owned_by_launcher() {
    let dir = test_dir("png");
    let source = dir.join("wide.png");
    ImageBuffer::from_pixel(320, 80, Rgba([255u8, 0, 0, 255]))
        .save(&source)
        .unwrap();
    let relative = import_image(&dir, GameId(9), &source).unwrap();
    assert_eq!(relative, PathBuf::from("launcher_assets/game_images/9.png"));
    let imported = image::open(dir.join(&relative)).unwrap();
    assert_eq!(
        imported.dimensions(),
        (NORMALIZED_SIDE_PX, NORMALIZED_SIDE_PX)
    );
    assert_eq!(imported.get_pixel(0, 0).0[3], 0);
    assert_eq!(imported.get_pixel(256, 256).0, [255, 0, 0, 255]);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn corrupt_oversized_and_orientation_paths_are_bounded() {
    let dir = test_dir("invalid");
    let corrupt = dir.join("bad.png");
    fs::write(&corrupt, b"not an image").unwrap();
    assert!(import_image(&dir, GameId(1), &corrupt).is_err());
    let huge = dir.join("huge.jpg");
    let file = fs::File::create(&huge).unwrap();
    file.set_len(MAX_ENCODED_BYTES + 1).unwrap();
    assert!(import_image(&dir, GameId(1), &huge).is_err());

    let mut source = RgbaImage::new(2, 1);
    source.put_pixel(0, 0, Rgba([1, 2, 3, 255]));
    let rotated = normalize(DynamicImage::ImageRgba8(source), Orientation::Rotate90);
    assert_eq!(
        rotated.dimensions(),
        (NORMALIZED_SIDE_PX, NORMALIZED_SIDE_PX)
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cleanup_only_removes_unreferenced_launcher_owned_assets() {
    let dir = test_dir("cleanup");
    let relative = PathBuf::from("launcher_assets/game_images/3.png");
    fs::create_dir_all(dir.join(IMAGE_DIRECTORY)).unwrap();
    fs::write(dir.join(&relative), b"image").unwrap();
    assert!(!remove_if_unreferenced(&dir, &relative, [relative.clone()]).unwrap());
    assert!(remove_if_unreferenced(&dir, &relative, Vec::<PathBuf>::new()).unwrap());
    assert!(remove_if_unreferenced(&dir, Path::new("../outside.png"), []).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn background_is_copied_owned_and_survives_source_deletion() {
    let dir = test_dir("background");

    let source = dir.join("photo.png");
    ImageBuffer::from_pixel(1000, 3000, Rgba([10u8, 20, 30, 255]))
        .save(&source)
        .unwrap();

    let relative = import_background(&dir, &source).unwrap();
    assert!(relative.starts_with(BACKGROUND_DIRECTORY), "{relative:?}");
    let name = relative.file_name().unwrap().to_str().unwrap();
    assert_eq!(name.len(), 20, "{name}: 16 hex digits and .png");
    assert!(name.ends_with(".png") && name[..16].bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(is_launcher_owned_background(&relative));
    assert!(!is_launcher_owned_background(&source));

    let stored = image::open(dir.join(&relative)).unwrap();

    let (w, h) = stored.dimensions();
    assert_eq!(h, BACKGROUND_MAX_EDGE);
    assert!(w < h && w <= BACKGROUND_MAX_EDGE);
    assert!((f64::from(w) / f64::from(h) - 1000.0 / 3000.0).abs() < 0.01);

    fs::remove_file(&source).unwrap();
    assert!(load_background(&dir, &relative).is_ok());

    assert!(load_background(&dir, &source).is_err());

    assert!(load_background(&dir, Path::new("../outside.png")).is_err());

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn backgrounds_are_named_after_their_content_and_the_unreferenced_copy_is_removed() {
    let dir = test_dir("content-addressed");
    let first = dir.join("first.png");
    ImageBuffer::from_pixel(40, 30, Rgba([10u8, 20, 30, 255]))
        .save(&first)
        .unwrap();
    let second = dir.join("second.png");
    ImageBuffer::from_pixel(40, 30, Rgba([200u8, 20, 30, 255]))
        .save(&second)
        .unwrap();

    let first_copy = import_background(&dir, &first).unwrap();
    let second_copy = import_background(&dir, &second).unwrap();
    assert_ne!(first_copy, second_copy);
    assert!(dir.join(&first_copy).is_file() && dir.join(&second_copy).is_file());
    assert_eq!(import_background(&dir, &first).unwrap(), first_copy);
    assert_eq!(
        fs::read_dir(dir.join(BACKGROUND_DIRECTORY))
            .unwrap()
            .count(),
        2
    );

    assert!(!remove_if_unreferenced(&dir, &second_copy, [second_copy.clone()]).unwrap());
    assert!(remove_if_unreferenced(&dir, &first_copy, [second_copy.clone()]).unwrap());
    assert!(!dir.join(&first_copy).exists() && dir.join(&second_copy).is_file());

    assert!(!remove_if_unreferenced(&dir, &first_copy, Vec::<PathBuf>::new()).unwrap());
    assert!(remove_if_unreferenced(&dir, Path::new("launcher_assets/other.png"), []).is_err());

    let fixed = PathBuf::from("launcher_assets/backgrounds/background.png");
    fs::copy(dir.join(&second_copy), dir.join(&fixed)).unwrap();
    assert!(is_launcher_owned_background(&fixed));
    assert!(load_background(&dir, &fixed).is_ok());
    assert!(remove_if_unreferenced(&dir, &fixed, []).unwrap());
    fs::remove_dir_all(dir).unwrap();
}
