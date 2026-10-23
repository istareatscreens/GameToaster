use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn flatten_locale(
    value: &serde_json::Value,
    prefix: &str,
    output: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let object = value.as_object().expect("locale nodes must be objects");
    for (key, child) in object {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if child.is_object() {
            flatten_locale(child, &path, output);
            continue;
        }
        let text = child
            .as_str()
            .unwrap_or_else(|| panic!("locale leaf `{path}` must be a string"));
        let mut placeholders = BTreeSet::new();
        let mut remainder = text;
        while let Some(start) = remainder.find("%{") {
            remainder = &remainder[start + 2..];
            let end = remainder
                .find('}')
                .unwrap_or_else(|| panic!("locale leaf `{path}` has an open placeholder"));
            let name = &remainder[..end];
            assert!(
                !name.is_empty()
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
                "locale leaf `{path}` has invalid placeholder `%{{{name}}}`"
            );
            placeholders.insert(name.to_string());
            remainder = &remainder[end + 1..];
        }
        assert!(
            output.insert(path.clone(), placeholders).is_none(),
            "duplicate locale leaf `{path}`"
        );
    }
}

#[test]
fn every_locale_has_identical_keys_and_placeholders() {
    let locales = [
        ("en", include_str!("../../../../../locales/en.json")),
        ("ja", include_str!("../../../../../locales/ja.json")),
        (
            "zh-Hans",
            include_str!("../../../../../locales/zh-Hans.json"),
        ),
        (
            "zh-Hant",
            include_str!("../../../../../locales/zh-Hant.json"),
        ),
    ];
    let mut expected = None;
    for (locale, raw) in locales {
        let value: serde_json::Value = serde_json::from_str(raw)
            .unwrap_or_else(|error| panic!("invalid {locale} locale JSON: {error}"));
        let mut flattened = BTreeMap::new();
        flatten_locale(&value, "", &mut flattened);
        if let Some(reference) = &expected {
            assert_eq!(
                &flattened, reference,
                "locale `{locale}` differs in keys or placeholder names"
            );
        } else {
            expected = Some(flattened);
        }
    }
}

#[test]
fn maps_bcp47_tags_to_our_locales() {
    assert_eq!(map_bcp47("ja-JP"), "ja");
    assert_eq!(map_bcp47("zh-Hant-TW"), "zh-Hant");
    assert_eq!(map_bcp47("zh-TW"), "zh-Hant");
    assert_eq!(map_bcp47("zh-Hans-CN"), "zh-Hans");
    assert_eq!(map_bcp47("zh-CN"), "zh-Hans");
    assert_eq!(map_bcp47("en-US"), "en");
    assert_eq!(map_bcp47("fr-FR"), "en");
}

#[test]
fn translations_actually_resolve_not_just_keys() {
    use rust_i18n::t;
    let _locale = crate::gui::common::harness::locale("en");
    assert_eq!(&*t!("settings.auto_hide"), "Auto hide on game launch");
    assert_eq!(&*t!("settings.main_display"), "Main display");
    assert_eq!(&*t!("action.add_game"), "Add Game");
    rust_i18n::set_locale("ja");
    assert_eq!(&*t!("settings.auto_hide"), "ゲーム起動時に自動で隠す");
    rust_i18n::set_locale("zh-Hans");
    assert_eq!(&*t!("action.add_game"), "添加游戏");
    rust_i18n::set_locale("zh-Hant");
    assert_eq!(&*t!("settings.main_display"), "主螢幕");
}

#[test]
fn dialog_keys_resolve_in_every_locale() {
    use rust_i18n::t;
    let _locale = crate::gui::common::harness::locale("en");
    let keys = [
        "title.game_settings",
        "plugins.choose_revision",
        "hotkeys.launch",
        "gs.add_store",
        "cfgmenu.hint",
        "picker.add_game",
        "common.cancel",
    ];
    for locale in ["en", "zh-Hans", "zh-Hant", "ja"] {
        rust_i18n::set_locale(locale);
        for key in keys {
            assert_ne!(&*t!(key), key, "missing `{key}` in `{locale}`");
        }
    }
}

#[test]
fn controller_keys_resolve_in_every_locale() {
    use rust_i18n::t;
    let _locale = crate::gui::common::harness::locale("en");

    let keys = [
        "controller.title",
        "controller.save",
        "controller.dialog.release_failed_title",
        "controller.dialog.release_failed_body",
        "controller.file_dialog.background_title",
        "controller.file_dialog.image_filter",
        "controller.validation.select_monitor",
        "controller.validation.hex",
        "controller.tray.toggle_main",
        "controller.behavior.heading",
        "controller.behavior.launch_on_start",
        "controller.behavior.open_on_os_startup",
        "controller.behavior.open_on_os_startup_hint",
        "controller.startup.adapter_missing_title",
        "controller.startup.adapter_missing_body",
        "controller.action.label",
        "controller.action.minimize",
        "controller.type.select",
        "controller.type.adx_hid",
        "controller.type.jvs_player",
        "controller.type.player1",
        "controller.test.start",
        "controller.test.auto_detect",
        "controller.test.passed",
        "controller.calib.heading",
        "controller.calib.auto",
        "controller.calib.bottom",
        "controller.calib.centered",
        "controller.background.artwork_heading",
        "controller.background.square_hint",
        "controller.screen.empty",
        "controller.screen.credits",
        "controller.card.enable",
        "controller.status.disconnected",
        "controller.status.connected",
    ];
    for locale in ["en", "zh-Hans", "zh-Hant", "ja"] {
        rust_i18n::set_locale(locale);
        for key in keys {
            assert_ne!(&*t!(key), key, "missing `{key}` in `{locale}`");
        }
    }
}

#[test]
fn resolved_prefers_known_saved_value() {
    assert_eq!(resolved_code("ja"), "ja");
    assert_eq!(resolved_code("zh-Hant"), "zh-Hant");

    assert!(is_known(resolved_code("")));
    assert!(is_known(resolved_code("xx")));
}

fn locale_texts() -> Vec<(&'static str, BTreeMap<String, String>)> {
    fn flatten(value: &serde_json::Value, prefix: &str, output: &mut BTreeMap<String, String>) {
        for (key, child) in value.as_object().expect("locale nodes must be objects") {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match child.as_str() {
                Some(text) => {
                    output.insert(path, text.to_owned());
                }
                None => flatten(child, &path, output),
            }
        }
    }
    [
        ("en", include_str!("../../../../../locales/en.json")),
        ("ja", include_str!("../../../../../locales/ja.json")),
        (
            "zh-Hans",
            include_str!("../../../../../locales/zh-Hans.json"),
        ),
        (
            "zh-Hant",
            include_str!("../../../../../locales/zh-Hant.json"),
        ),
    ]
    .into_iter()
    .map(|(locale, raw)| {
        let mut texts = BTreeMap::new();
        flatten(&serde_json::from_str(raw).unwrap(), "", &mut texts);
        (locale, texts)
    })
    .collect()
}

struct Literal {
    value: String,
    start: usize,
}

fn string_literals(text: &str) -> Vec<Literal> {
    let bytes = text.as_bytes();
    let identifier = |index: usize| {
        index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_')
    };
    let mut literals = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let rest = &text[index..];
        if rest.starts_with("//") {
            index += rest.find('\n').unwrap_or(rest.len());
        } else if rest.starts_with("/*") {
            index += rest.find("*/").map_or(rest.len(), |end| end + 2);
        } else if (rest.starts_with("r\"") || rest.starts_with("r#") || rest.starts_with("br"))
            && !identifier(index)
            && rest
                .trim_start_matches(['b', 'r'])
                .trim_start_matches('#')
                .starts_with('"')
        {
            let prefix = rest.len() - rest.trim_start_matches(['b', 'r']).len();
            let hashes = rest[prefix..].len() - rest[prefix..].trim_start_matches('#').len();
            let open = index + prefix + hashes + 1;
            let close = format!("\"{}", "#".repeat(hashes));
            let end = open + text[open..].find(&close).expect("unterminated raw string");
            literals.push(Literal {
                value: text[open..end].to_owned(),
                start: index,
            });
            index = end + close.len();
        } else if bytes[index] == b'"' || (rest.starts_with("b\"") && !identifier(index)) {
            let open = index + if bytes[index] == b'b' { 2 } else { 1 };
            let mut end = open;
            while bytes[end] != b'"' {
                end += if bytes[end] == b'\\' { 2 } else { 1 };
            }
            literals.push(Literal {
                value: text[open..end].to_owned(),
                start: index,
            });
            index = end + 1;
        } else if bytes[index] == b'\'' {
            let after = &text[index + 1..];
            if after.starts_with('\\') {
                index += 1 + after[2..].find('\'').map_or(after.len(), |end| end + 3);
            } else {
                let width = after.chars().next().map_or(0, char::len_utf8);
                index += if after[width..].starts_with('\'') {
                    width + 2
                } else {
                    1
                };
            }
        } else {
            index += 1;
        }
    }
    literals
}

fn is_translation_key(text: &str, start: usize) -> bool {
    let before = text[..start].trim_end();
    ["t!(", "message!("].iter().any(|call| {
        before.strip_suffix(call).is_some_and(|head| {
            !head
                .chars()
                .next_back()
                .is_some_and(|last| last.is_alphanumeric() || last == '_')
        })
    })
}

fn sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![root.clone()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path.strip_prefix(&root).unwrap();
                let relative = relative.to_string_lossy().replace('\\', "/");
                files.push((relative, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_translation_key_in_the_source_exists_in_every_locale() {
    let locales = locale_texts();
    let mut checked = 0;
    for (file, text) in sources() {
        for literal in string_literals(&text) {
            if !is_translation_key(&text, literal.start) {
                continue;
            }
            checked += 1;
            for (locale, texts) in &locales {
                assert!(
                    texts.contains_key(&literal.value),
                    "`{}` ({file}) is missing from {locale}",
                    literal.value
                );
            }
        }
    }
    assert!(
        checked > 400,
        "only {checked} keys found: the scan is broken"
    );
}

#[test]
fn every_locale_key_is_used_by_the_source() {
    let mut used = BTreeSet::new();
    for (_, text) in sources() {
        used.extend(
            string_literals(&text)
                .into_iter()
                .map(|literal| literal.value),
        );
    }
    let (_, english) = &locale_texts()[0];
    let unused: Vec<&String> = english.keys().filter(|key| !used.contains(*key)).collect();
    assert!(unused.is_empty(), "locale keys no source uses: {unused:?}");
}

const TECHNICAL_TOKENS: &[&str] = &[
    "ADX",
    "HID",
    "IO4",
    "JVS",
    "LED",
    "VID",
    "PID",
    "mxjvs",
    "COM1",
    "COM21",
    "COM65535",
    "COM1-COM65535",
    "Ctrl",
    "Alt",
    "Shift",
    "Esc",
    "X",
    "exe",
    "bat",
    "cmd",
    "PNG",
    "JPEG",
    "ZIP",
    "TOML",
    "MD5",
    "SHA-256",
    "OpenGL",
    "GameToaster",
    "Windows",
    "Microsoft",
    "pluginVersion",
    "true",
    "false",
    "ID",
    "DPI",
    "px",
    "v",
    "x",
    "OK",
    "UI",
];

fn ascii_words(text: &str) -> Vec<String> {
    let mut plain = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("%{") {
        plain.push_str(&rest[..start]);
        plain.push(' ');
        rest = &rest[start + rest[start..].find('}').unwrap() + 1..];
    }
    plain.push_str(rest);
    let word_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
    plain
        .split(|c: char| !word_char(c))
        .filter(|token| token.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter(|token| {
            !(token.ends_with(".exe") || token.ends_with(".bat") || token.ends_with(".cmd"))
        })
        .map(|token| token.trim_end_matches('.').to_owned())
        .collect()
}

#[test]
fn japanese_and_chinese_texts_hold_no_english_word() {
    for (locale, texts) in locale_texts().into_iter().skip(1) {
        for (key, text) in texts {
            for word in ascii_words(&text) {
                assert!(
                    TECHNICAL_TOKENS.contains(&word.as_str()),
                    "{locale} `{key}` holds the English word `{word}`: {text}"
                );
            }
        }
    }
}

const GUI_LAYER: &[&str] = &[
    "main.rs",
    "gui/common/",
    "gui/main_window/app/",
    "gui/main_window/ui/",
    "gui/main_window/platform/",
    "gui/game_launcher_screen/ui/",
    "gui/game_launcher_screen/launcher/",
];

const PLATFORM_LAYER: &str = "gui/main_window/platform/";

const UNTRANSLATED: &[(&str, &str, &str)] = &[
    (
        "gui/main_window/app/run.rs",
        "<non-string panic payload>",
        "a crash-log value",
    ),
    (
        "gui/main_window/ui/dialogs/manifest_editor.rs",
        "pluginId = {}",
        "a manifest field, shown as written",
    ),
    (
        "gui/main_window/ui/dialogs/plugin_action.rs",
        "{kind}: {}",
        "a format with no words",
    ),
    (
        "gui/main_window/ui/dialogs/plugin_library.rs",
        "SHA-256  {}",
        "a hash name",
    ),
    (
        "gui/game_launcher_screen/launcher/settings.rs",
        "GameToaster Calibration",
        "a viewport title, never shown",
    ),
    (
        "gui/game_launcher_screen/launcher/window.rs",
        "GameToaster Controller Launcher",
        "a viewport title the cabinet harness matches",
    ),
    (
        "gui/main_window/ui/i18n.rs",
        "English",
        "the language's own name in the language list",
    ),
    (
        "gui/main_window/ui/views/settings.rs",
        "English",
        "the language list's fallback, English's own name",
    ),
    (
        "gui/main_window/platform/store_apps.rs",
        "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;…",
        "a PowerShell script passed to Windows, never shown",
    ),
];

fn matches_exemption(value: &str, exempt: &str) -> bool {
    match exempt.strip_suffix('…') {
        Some(prefix) => value.starts_with(prefix),
        None => value == exempt,
    }
}

const LOG_CALLS: &[&str] = &[
    "diagnostics.error(",
    "diagnostics.transition(",
    ".crash(",
    "log.event(",
    "eprintln!(",
    "println!(",
    "panic!(",
    "unreachable!(",
    "assert!(",
    "assert_eq!(",
    ".expect(",
];

fn reads_like_text(value: &str) -> bool {
    let sentence = value.contains(' ')
        && value
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|word| word.len() >= 3);
    let mut letters = value.chars();
    let word = value.len() >= 3
        && letters.next().is_some_and(|c| c.is_ascii_uppercase())
        && letters.all(|c| c.is_ascii_lowercase());
    sentence || word
}

fn is_error_text(text: &str, start: usize) -> bool {
    let from = (0..=start.saturating_sub(160))
        .rev()
        .find(|&index| text.is_char_boundary(index))
        .unwrap_or(0);
    let before: String = text[from..start].split_whitespace().collect();
    let before = before.strip_suffix("format!(").unwrap_or(&before);
    let before = before.strip_suffix('{').unwrap_or(before);
    before.ends_with("Err(")
        || before.ends_with("ok_or(")
        || before.ends_with("ok_or_else(||")
        || before.rsplit_once("map_err(|").is_some_and(|(_, closure)| {
            closure
                .strip_suffix('|')
                .is_some_and(|name| name.chars().all(|c| c.is_alphanumeric() || c == '_'))
        })
}

fn statement_head(text: &str, start: usize) -> String {
    let head = &text[..start];
    let from = head.rfind([';', '{', '}']).map_or(0, |index| index + 1);
    head[from..].split_whitespace().collect()
}

#[test]
fn the_gui_layer_has_no_untranslated_sentence() {
    let mut found = Vec::new();
    let mut exempt_seen = BTreeSet::new();
    for (file, text) in sources() {
        if !GUI_LAYER.iter().any(|layer| file.starts_with(layer)) {
            continue;
        }
        for literal in string_literals(&text) {
            if !reads_like_text(&literal.value) || is_translation_key(&text, literal.start) {
                continue;
            }
            let head = statement_head(&text, literal.start);
            if LOG_CALLS.iter().any(|call| head.contains(call)) {
                continue;
            }
            if file.starts_with(PLATFORM_LAYER) && is_error_text(&text, literal.start) {
                continue;
            }
            match UNTRANSLATED
                .iter()
                .find(|(path, value, _)| *path == file && matches_exemption(&literal.value, value))
            {
                Some(exempt) => {
                    exempt_seen.insert(*exempt);
                }
                None => found.push(format!("{file}: {:?}", literal.value)),
            }
        }
    }
    assert!(
        found.is_empty(),
        "untranslated sentences in the GUI layer: {found:#?}"
    );
    let stale: Vec<_> = UNTRANSLATED
        .iter()
        .filter(|exempt| !exempt_seen.contains(*exempt))
        .collect();
    assert!(
        stale.is_empty(),
        "exemptions with no literal left: {stale:?}"
    );
}

#[test]
fn the_add_game_busy_label_and_file_filters_read_in_simplified_chinese() {
    use rust_i18n::t;
    let _locale = crate::gui::common::harness::locale("zh-Hans");
    assert_eq!(t!("busy.adding_game"), "正在验证并添加游戏…");
    assert_eq!(t!("picker.filter_executable"), "程序或脚本");
    assert_eq!(t!("picker.filter_all"), "所有文件");
}

#[test]
fn a_bad_controller_option_reads_in_traditional_chinese() {
    let _locale = crate::gui::common::harness::locale("zh-Hant");
    let mut config = crate::game_launcher_screen::default_controller_config("adx-hid").unwrap();
    config.options.insert("vid".into(), "zz".into());
    assert_eq!(
        crate::game_launcher_screen::validate_controller_config(&config).unwrap_err(),
        "控制器選項「vid」必須是 16 位元十六進位值"
    );
}

#[test]
fn converted_texts_read_in_english_exactly_as_before() {
    use rust_i18n::t;
    let _locale = crate::gui::common::harness::locale("en");

    assert_eq!(
        crate::launcher_gui::install::validate_selected_executable(std::path::Path::new(
            "relative.exe"
        ))
        .unwrap_err(),
        "selected launchable path must be absolute: relative.exe"
    );
    let mut config = crate::game_launcher_screen::default_controller_config("adx-hid").unwrap();
    config.options.insert("vid".into(), "zz".into());
    assert_eq!(
        crate::game_launcher_screen::validate_controller_config(&config).unwrap_err(),
        "controller option 'vid' must be a hexadecimal 16-bit value"
    );

    assert_eq!(t!("busy.adding_game"), "Validating and adding game…");
    assert_eq!(
        t!("errors.launch_failed", game => "Game"),
        "Could not launch Game"
    );
    assert_eq!(
        t!("settings.monitor_label_primary", number => 2, width => 1920, height => 1080),
        "Monitor 2 — 1920×1080 (primary)"
    );
    assert_eq!(
        t!("remove.record_only"),
        "This removes the game record. Assigned plugins are removed through the plugin lifecycle."
    );
    assert_eq!(t!("startup.error_title"), "GameToaster error");
}
