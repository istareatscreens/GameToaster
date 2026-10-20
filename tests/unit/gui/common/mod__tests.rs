use std::{fs, path::Path};

struct Rule {
    construct: &'static str,
    patterns: &'static [&'static str],

    owner: Option<&'static str>,
}

const RULES: &[Rule] = &[
    Rule {
        construct: "button",
        patterns: &[
            ".button(",
            ".small_button(",
            "Button::new(",
            "filled_button(",
        ],
        owner: Some("common/components/button.rs"),
    },
    Rule {
        construct: "checkbox or radio",
        patterns: &[
            ".checkbox(",
            ".radio_value(",
            "Checkbox::new(",
            "RadioButton::new(",
        ],
        owner: Some("common/components/toggle.rs"),
    },
    Rule {
        construct: "combo box",
        patterns: &["ComboBox::"],
        owner: Some("common/components/combo.rs"),
    },
    Rule {
        construct: "window or modal",
        patterns: &["Window::new(", "Modal::new("],
        owner: Some("common/components/dialog.rs"),
    },
    Rule {
        construct: "frame",
        patterns: &[
            "Frame::new(",
            "Frame::default(",
            "Frame::group(",
            "Frame::canvas(",
            "Frame::dark_canvas(",
            "Frame::window(",
            "Frame::menu(",
            "Frame::popup(",
            "Frame::central_panel(",
            "Frame::side_top_panel(",
        ],
        owner: Some("common/theme/frames.rs"),
    },
];

const FONT_SIZE_CALLS: &[&str] = &[
    ".size(",
    "FontId::proportional(",
    "FontId::monospace(",
    "FontId::new(",
];

const CLICKABLE: &[&str] = &[
    "Sense::click(",
    "Sense::click_and_drag(",
    "Sense::drag(",
    "Sense::CLICK",
    "Sense::DRAG",
];
const CLICKABLE_OWNER: &str = "common/components/";
const CLICKABLE_REGIONS: &[(&str, usize)] = &[
    ("main_window/ui/views/mod.rs", 1),
    ("main_window/ui/views/game_list.rs", 1),
    ("main_window/ui/dialogs/hotkeys.rs", 1),
    (
        "game_launcher_screen/ui/layouts/washing_machine/affordances.rs",
        1,
    ),
];

fn gui_sources() -> Vec<(String, String)> {
    let gui_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("gui");
    let mut pending = vec![gui_root.clone()];
    let mut sources = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                let relative = path
                    .strip_prefix(&gui_root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let code = fs::read_to_string(&path)
                    .unwrap()
                    .lines()
                    .map(|line| line.split("//").next().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join("\n");
                sources.push((relative, code));
            }
        }
    }
    sources.sort();
    sources
}

fn occurrences<'a>(source: &'a str, pattern: &'a str) -> impl Iterator<Item = usize> + 'a {
    let starts_with_name = pattern.starts_with(|c: char| c.is_alphanumeric() || c == '_');
    source
        .match_indices(pattern)
        .map(|(index, _)| index)
        .filter(move |&index| {
            !starts_with_name
                || !source[..index]
                    .chars()
                    .next_back()
                    .is_some_and(|before| before.is_alphanumeric() || before == '_')
        })
}

#[test]
fn every_construct_is_built_only_by_its_owner() {
    for (path, source) in gui_sources() {
        for rule in RULES {
            if rule.owner == Some(path.as_str()) {
                continue;
            }
            for pattern in rule.patterns {
                assert!(
                    occurrences(&source, pattern).next().is_none(),
                    "{path} builds a {} with {pattern} instead of using {}",
                    rule.construct,
                    rule.owner.unwrap_or("the shared layer")
                );
            }
        }
        for call in FONT_SIZE_CALLS {
            for index in occurrences(&source, call) {
                let argument = source[index + call.len()..].trim_start();
                assert!(
                    !argument.starts_with(|c: char| c.is_ascii_digit()),
                    "{path} sets a literal font size with {call}: use theme::font_size"
                );
            }
        }
        if path.starts_with(CLICKABLE_OWNER) {
            continue;
        }
        let regions: usize = CLICKABLE
            .iter()
            .map(|pattern| occurrences(&source, pattern).count())
            .sum();
        let allowed = CLICKABLE_REGIONS
            .iter()
            .find(|(file, _)| *file == path)
            .map_or(0, |(_, count)| *count);
        assert_eq!(
            regions, allowed,
            "{path} paints {regions} clickable regions; controls come from components"
        );
    }
}

#[test]
fn common_gui_owns_theme_without_feature_dependencies() {
    let gui_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("gui");
    assert!(gui_root.join("common/theme/mod.rs").is_file());
    assert!(!gui_root.join("main_window/ui/theme.rs").exists());

    for (path, source) in gui_sources() {
        if path.starts_with("common/") {
            for forbidden in [
                "crate::launcher_gui",
                "crate::gui::main_window",
                "crate::game_launcher_screen",
                "crate::gui::game_launcher_screen",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{path} reverses the common GUI dependency through {forbidden}"
                );
            }
        }
        assert!(
            !source.contains("launcher_gui::theme"),
            "{path} uses removed theme ownership"
        );
        assert!(
            !source.contains("main_window::ui::theme"),
            "{path} uses feature-owned theme"
        );
    }
}
