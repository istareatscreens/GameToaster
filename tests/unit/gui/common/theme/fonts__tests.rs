use super::*;

use std::collections::BTreeSet;

fn folders() -> Vec<PathBuf> {
    vec![PathBuf::from("system-fonts"), PathBuf::from("user-fonts")]
}

fn select(present: &[&str]) -> Vec<(&'static str, PathBuf)> {
    let present: BTreeSet<PathBuf> = present.iter().map(PathBuf::from).collect();
    select_cjk_fonts(&folders(), |path| present.contains(path))
}

#[test]
fn every_script_loads_its_first_present_candidate() {
    assert_eq!(
        select(&[
            "system-fonts/msyh.ttc",
            "system-fonts/msjh.ttc",
            "system-fonts/meiryo.ttc",
            "system-fonts/YuGothM.ttc",
        ]),
        vec![
            ("cjk_sc", PathBuf::from("system-fonts/msyh.ttc")),
            ("cjk_tc", PathBuf::from("system-fonts/msjh.ttc")),
            ("cjk_jp", PathBuf::from("system-fonts/meiryo.ttc")),
        ]
    );
}

#[test]
fn japanese_falls_back_to_yu_gothic_without_the_supplemental_fonts() {
    assert_eq!(
        select(&["system-fonts/msyh.ttc", "system-fonts/YuGothM.ttc"]),
        vec![
            ("cjk_sc", PathBuf::from("system-fonts/msyh.ttc")),
            ("cjk_jp", PathBuf::from("system-fonts/YuGothM.ttc")),
        ]
    );
}

#[test]
fn a_font_installed_for_the_user_only_is_found_in_the_per_user_folder() {
    assert_eq!(
        select(&["system-fonts/YuGothM.ttc", "user-fonts/meiryo.ttc"]),
        vec![("cjk_jp", PathBuf::from("user-fonts/meiryo.ttc"))]
    );
}

#[test]
fn no_cjk_font_at_all_selects_nothing_and_fails_nothing() {
    assert!(select(&[]).is_empty());
    assert!(select_cjk_fonts(&[], |_| true).is_empty());
}

#[test]
fn candidates_are_file_names_looked_up_in_the_fonts_folders() {
    for (family, candidates) in CJK_CANDIDATES {
        assert!(family.starts_with("cjk_"));
        for name in candidates {
            assert!(name.ends_with(".ttc"), "{name}");
            assert!(!name.contains(['\\', '/', ':']), "{name} names a folder");
        }
    }
}

fn applied_styles() -> BTreeMap<TextStyle, FontId> {
    let context = egui::Context::default();
    crate::gui::common::theme::apply(&context);
    context.global_style().text_styles.clone()
}

#[test]
fn every_text_style_maps_onto_the_type_scale() {
    let styles = applied_styles();
    let expected = [
        (TextStyle::Small, 13.0, FontFamily::Proportional),
        (TextStyle::Body, 17.0, FontFamily::Proportional),
        (TextStyle::Button, 17.0, FontFamily::Proportional),
        (TextStyle::Monospace, 15.0, FontFamily::Monospace),
        (TextStyle::Heading, 24.0, FontFamily::Proportional),
    ];
    for (style, size, family) in expected {
        assert_eq!(styles[&style], FontId::new(size, family), "{style:?}");
    }
}

#[test]
fn small_text_is_no_longer_eguis_nine_points() {
    let context = egui::Context::default();
    assert_eq!(TextStyle::Small.resolve(&context.global_style()).size, 9.0);
    crate::gui::common::theme::apply(&context);
    assert_eq!(
        TextStyle::Small.resolve(&context.global_style()).size,
        font_size::SMALL
    );
}

#[test]
fn the_type_scale_is_strictly_increasing() {
    let scale = [
        font_size::TINY,
        font_size::SMALL,
        font_size::CAPTION,
        font_size::COMPACT,
        font_size::MEDIUM,
        font_size::BODY,
        font_size::LARGE,
        font_size::TITLE,
        font_size::ICON,
        font_size::HEADING,
    ];
    assert_eq!(
        scale,
        [12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 20.0, 22.0, 24.0]
    );
    assert!(scale.windows(2).all(|pair| pair[0] < pair[1]));
}
