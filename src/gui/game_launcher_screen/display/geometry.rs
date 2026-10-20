use crate::game_launcher_screen::OrientationProfile;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui::game_launcher_screen) struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub(in crate::gui::game_launcher_screen) fn resolve_profile(
    configured: OrientationProfile,
    monitor: PixelRect,
) -> OrientationProfile {
    match configured {
        OrientationProfile::Auto if monitor.height > monitor.width => {
            OrientationProfile::CabinetPortrait
        }
        OrientationProfile::Auto => OrientationProfile::ProductionLandscape,
        explicit => explicit,
    }
}
