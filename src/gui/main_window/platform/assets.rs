use std::path::Path;

pub const ICON_NAME: &str = "toaster.png";

pub fn icon_rgba(launcher_dir: &Path, size: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let external = launcher_dir.join(ICON_NAME);
    let bytes = if external.is_file() {
        std::fs::read(&external)
            .map_err(|error| format!("failed to read {}: {error}", external.display()))?
    } else {
        include_bytes!("../../../../assets/toaster.png").to_vec()
    };
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("failed to decode GameToaster icon: {error}"))?
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    Ok((image.into_raw(), size, size))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/platform/assets__tests.rs"]
mod tests;
