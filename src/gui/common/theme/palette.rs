use eframe::egui::{Color32, Stroke};

pub const SKY_TOP: Color32 = Color32::from_rgb(0x8C, 0xE7, 0xE2);
pub const SKY_BOTTOM: Color32 = Color32::from_rgb(0xCB, 0xEF, 0xFB);
pub const CANVAS: Color32 = SKY_TOP;

pub const LIST_PANEL: Color32 = Color32::from_rgb(0xC5, 0xCF, 0xFE);
pub const ROW_CREAM: Color32 = Color32::from_rgb(0xFF, 0xF4, 0xE0);
pub const ICON_TILE: Color32 = Color32::from_rgb(0xFF, 0xFB, 0xF2);
pub const BANNER_LAVENDER: Color32 = Color32::from_rgb(0xDC, 0xE0, 0xFB);

pub const TOPBAR_CORAL: Color32 = Color32::from_rgb(0xFF, 0x85, 0x95);
pub const PRIMARY_ORANGE: Color32 = Color32::from_rgb(0xFF, 0xA1, 0x6C);
pub const SALMON: Color32 = Color32::from_rgb(0xE5, 0x91, 0x94);

pub const RAINBOW_ROSE: Color32 = Color32::from_rgb(0xEF, 0x76, 0x76);
pub const RAINBOW_PEACH: Color32 = Color32::from_rgb(0xFF, 0xB3, 0x7A);
pub const RAINBOW_GOLD: Color32 = Color32::from_rgb(0xF4, 0xD7, 0x76);
pub const RAINBOW_MINT: Color32 = Color32::from_rgb(0x9A, 0xDB, 0x9A);
pub const RAINBOW_TEAL: Color32 = Color32::from_rgb(0x66, 0xCB, 0xD0);
pub const RAINBOW_SKY: Color32 = Color32::from_rgb(0x86, 0xB6, 0xEE);

pub const SELECTED_CYAN: Color32 = Color32::from_rgb(0x4F, 0xC6, 0xE0);
pub const INACTIVE_LAVENDER: Color32 = Color32::from_rgb(0xDC, 0xE0, 0xF5);

pub const INK: Color32 = Color32::from_rgb(0x2C, 0x30, 0x48);
pub const MUTED: Color32 = Color32::from_rgb(0x6A, 0x70, 0x90);
pub const PATH_BLUE: Color32 = Color32::from_rgb(0x4A, 0x5B, 0xC0);
pub const ERROR: Color32 = Color32::from_rgb(0x9E, 0x2B, 0x3A);
pub const CLOSE_HOVER: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x7E);
pub const SCROLL_HANDLE: Color32 = Color32::from_rgb(0xA9, 0xB4, 0xE8);

pub const CALIBRATION_GUIDE: Color32 = Color32::from_rgb(235, 74, 50);

pub const DISABLED_TOGGLE: Color32 = Color32::from_gray(205);

pub const BORDER_COLOR: Color32 = Color32::from_rgb(0x9B, 0xA6, 0xCC);
pub const BORDER: Stroke = Stroke {
    width: 1.0,
    color: BORDER_COLOR,
};
