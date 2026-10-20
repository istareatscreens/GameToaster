use eframe::egui;

pub mod font_size;
mod fonts;
pub mod frames;
mod paint;
mod palette;
mod visuals;

pub use fonts::install_fonts;
pub use paint::{paint_glyph, paint_vertical_gradient};
pub use palette::*;
pub use visuals::{apply_scroll_bar_visuals, apply_widget_visuals, with_widget_visuals};

pub fn apply(context: &egui::Context) {
    context.set_visuals(visuals::launcher());
    let mut style = (*context.global_style()).clone();
    visuals::apply_spacing(&mut style.spacing);
    fonts::apply_text_styles(&mut style.text_styles);
    context.set_global_style(style);
}
