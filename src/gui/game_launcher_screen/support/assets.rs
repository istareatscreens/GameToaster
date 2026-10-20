use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{DynamicImage, GenericImage, ImageDecoder, ImageFormat, RgbaImage};
use rust_i18n::t;

use crate::launcher_gui::model::GameId;

pub(super) const IMAGE_DIRECTORY: &str = "launcher_assets/game_images";

pub(super) const BACKGROUND_DIRECTORY: &str = "launcher_assets/backgrounds";
pub(super) const NORMALIZED_SIDE_PX: u32 = 512;

pub(super) const BACKGROUND_MAX_EDGE: u32 = 2048;
pub(super) const MAX_ENCODED_BYTES: u64 = 16 * 1024 * 1024;
pub(super) const MAX_DECODED_SIDE: u32 = 8192;

enum Fit {
    LabelSquare,

    BackgroundCover,
}

pub(crate) fn import_image(
    launcher_dir: &Path,
    game_id: GameId,
    source: &Path,
) -> Result<PathBuf, String> {
    import_normalized(launcher_dir, source, Fit::LabelSquare, |_| {
        PathBuf::from(IMAGE_DIRECTORY).join(format!("{}.png", game_id.0))
    })
}

pub(crate) fn import_background(launcher_dir: &Path, source: &Path) -> Result<PathBuf, String> {
    import_normalized(launcher_dir, source, Fit::BackgroundCover, |encoded| {
        let content = crate::gui::common::hash::fnv1a_64(encoded);
        PathBuf::from(BACKGROUND_DIRECTORY).join(format!("{content:016x}.png"))
    })
}

fn import_normalized(
    launcher_dir: &Path,
    source: &Path,
    fit: Fit,
    name: impl FnOnce(&[u8]) -> PathBuf,
) -> Result<PathBuf, String> {
    let metadata = fs::metadata(source)
        .map_err(|error| format!("failed to inspect artwork {}: {error}", source.display()))?;
    if metadata.len() == 0 || metadata.len() > MAX_ENCODED_BYTES {
        return Err(format!(
            "artwork must be between 1 byte and {} MiB",
            MAX_ENCODED_BYTES / (1024 * 1024)
        ));
    }
    let reader = image::ImageReader::open(source)
        .map_err(|error| format!("failed to open artwork {}: {error}", source.display()))?
        .with_guessed_format()
        .map_err(|error| format!("failed to identify artwork {}: {error}", source.display()))?;
    if !matches!(reader.format(), Some(ImageFormat::Png | ImageFormat::Jpeg)) {
        return Err(t!("controller.validation.artwork_type").to_string());
    }
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| format!("failed to initialize artwork decoder: {error}"))?;
    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 || width > MAX_DECODED_SIDE || height > MAX_DECODED_SIDE {
        return Err(format!(
            "artwork dimensions {width}x{height} exceed the {}px limit",
            MAX_DECODED_SIDE
        ));
    }
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let decoded = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("failed to decode artwork: {error}"))?;
    let normalized = match fit {
        Fit::LabelSquare => normalize(decoded, orientation),
        Fit::BackgroundCover => normalize_cover(decoded, orientation, BACKGROUND_MAX_EDGE),
    };
    let mut encoded = Cursor::new(Vec::new());
    normalized
        .write_to(&mut encoded, ImageFormat::Png)
        .map_err(|error| format!("failed to encode normalized artwork: {error}"))?;

    let relative = name(encoded.get_ref());
    let target = launcher_dir.join(&relative);
    crate::support::fs::atomic_write(&target, encoded.get_ref())?;
    Ok(relative)
}

fn normalize(mut image: DynamicImage, orientation: Orientation) -> DynamicImage {
    image.apply_orientation(orientation);
    let thumbnail = image
        .thumbnail(NORMALIZED_SIDE_PX, NORMALIZED_SIDE_PX)
        .to_rgba8();
    let mut canvas = RgbaImage::new(NORMALIZED_SIDE_PX, NORMALIZED_SIDE_PX);
    let x = (NORMALIZED_SIDE_PX - thumbnail.width()) / 2;
    let y = (NORMALIZED_SIDE_PX - thumbnail.height()) / 2;
    let _ = canvas.copy_from(&thumbnail, x, y);
    DynamicImage::ImageRgba8(canvas)
}

fn normalize_cover(
    mut image: DynamicImage,
    orientation: Orientation,
    max_edge: u32,
) -> DynamicImage {
    image.apply_orientation(orientation);
    if image.width() > max_edge || image.height() > max_edge {
        image.resize(max_edge, max_edge, image::imageops::FilterType::Triangle)
    } else {
        image
    }
}

pub(crate) fn remove_if_unreferenced(
    launcher_dir: &Path,
    candidate: &Path,
    referenced: impl IntoIterator<Item = PathBuf>,
) -> Result<bool, String> {
    if candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
        || !(candidate.starts_with(IMAGE_DIRECTORY) || candidate.starts_with(BACKGROUND_DIRECTORY))
    {
        return Err(format!(
            "refusing to remove non-launcher asset path {}",
            candidate.display()
        ));
    }
    if referenced.into_iter().any(|path| path == candidate) {
        return Ok(false);
    }
    let target = launcher_dir.join(candidate);
    if target.is_file() {
        fs::remove_file(&target)
            .map_err(|error| format!("failed to remove artwork {}: {error}", target.display()))?;
        return Ok(true);
    }
    Ok(false)
}

pub(in crate::gui::game_launcher_screen) fn load_rgba_with_hash(
    launcher_dir: &Path,
    relative: &Path,
) -> Result<(DynamicImage, u64), String> {
    if relative.is_absolute() || !relative.starts_with(IMAGE_DIRECTORY) {
        return Err("controller artwork path is not launcher-owned".into());
    }
    let bytes = fs::read(launcher_dir.join(relative))
        .map_err(|error| format!("failed to read controller artwork: {error}"))?;
    let hash = crate::gui::common::hash::fnv1a_64(&bytes);
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("failed to load controller artwork: {error}"))?;
    Ok((image, hash))
}

pub(in crate::gui::game_launcher_screen) fn is_launcher_owned_background(path: &Path) -> bool {
    !path.is_absolute()
        && path.starts_with(BACKGROUND_DIRECTORY)
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

pub(in crate::gui::game_launcher_screen) fn load_background(
    launcher_dir: &Path,
    path: &Path,
) -> Result<DynamicImage, String> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else if is_launcher_owned_background(path) {
        launcher_dir.join(path)
    } else {
        return Err("background image path is not launcher-owned".into());
    };
    let bytes =
        fs::read(&full).map_err(|error| format!("failed to read background image: {error}"))?;
    image::load_from_memory(&bytes)
        .map_err(|error| format!("failed to load background image: {error}"))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/support/assets__tests.rs"]
mod tests;
