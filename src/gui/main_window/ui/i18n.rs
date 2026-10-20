use rust_i18n::set_locale;

pub struct Locale {
    pub code: &'static str,
    pub native_name: &'static str,
}

pub const LOCALES: [Locale; 4] = [
    Locale {
        code: "en",
        native_name: "English",
    },
    Locale {
        code: "zh-Hans",
        native_name: "简体中文",
    },
    Locale {
        code: "zh-Hant",
        native_name: "繁體中文",
    },
    Locale {
        code: "ja",
        native_name: "日本語",
    },
];

pub fn is_known(code: &str) -> bool {
    LOCALES.iter().any(|locale| locale.code == code)
}

pub fn resolved_code(saved: &str) -> &'static str {
    if let Some(locale) = LOCALES.iter().find(|locale| locale.code == saved) {
        return locale.code;
    }
    detect_os_locale()
}

pub fn apply(code: &str) {
    set_locale(if is_known(code) { code } else { "en" });
}

pub fn detect_os_locale() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    let mut buffer = [0u16; 85];
    let tag = unsafe {
        let written = GetUserDefaultLocaleName(&mut buffer);
        if written <= 1 {
            String::new()
        } else {
            String::from_utf16_lossy(&buffer[..(written - 1) as usize])
        }
    };
    map_bcp47(&tag)
}

pub fn map_bcp47(tag: &str) -> &'static str {
    let tag = tag.to_ascii_lowercase();
    if tag.starts_with("ja") {
        return "ja";
    }
    if tag.starts_with("zh") {
        if tag.contains("hant") || tag.contains("-tw") || tag.contains("-hk") || tag.contains("-mo")
        {
            return "zh-Hant";
        }
        return "zh-Hans";
    }
    "en"
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/ui/i18n__tests.rs"]
mod tests;
