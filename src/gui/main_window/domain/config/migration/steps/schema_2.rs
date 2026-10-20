use std::collections::HashMap;

use configparser::ini::Ini;

use super::super::chain::Findings;

pub(in super::super) const VERSION: u32 = 2;

const CONTROLLER_SECTION: &str = "game_launcher_screen";
const SCHEMA_2_KEYS: [&str; 6] = [
    "controller_adapter",
    "vid",
    "pid",
    "jvs_player",
    "jvs_endpoint",
    "jvs_hardware_validated",
];

const SCHEMA_3_KEYS: [&str; 3] = [
    "controller_driver",
    "launcher_layout",
    "reopen_on_monitor_return",
];
const SCHEMA_3_OPTION_PREFIX: &str = "controller_option.";

pub(in super::super) fn upgrade(document: &mut Ini, _: &mut Findings) -> Result<(), String> {
    let Some(values) = document.get_mut_map().get_mut(CONTROLLER_SECTION) else {
        return Ok(());
    };
    let mut old: HashMap<&str, String> = HashMap::new();
    for key in SCHEMA_2_KEYS {
        if let Some(value) = values.remove(key).flatten() {
            let value = value.trim();
            if !value.is_empty() {
                old.insert(key, value.to_string());
            }
        }
    }
    values.retain(|key, _| {
        !SCHEMA_3_KEYS.contains(&key.as_str()) && !key.starts_with(SCHEMA_3_OPTION_PREFIX)
    });
    let driver = match old.get("controller_adapter") {
        None => "adx-io4",
        Some(adapter) => match adapter.to_ascii_lowercase().as_str() {
            "adx-io4" | "io4" => "adx-io4",
            "adx-hid" => "adx-hid",
            "jvs" => "jvs",
            _ => return Err(format!("unknown controller adapter '{adapter}'")),
        },
    };
    let mut set = |option: &str, value: String| {
        values.insert(format!("controller_option.{option}"), Some(value));
    };
    if driver == "jvs" {
        if let Some(player) = old.get("jvs_player") {
            set("player", jvs_player(player)?.to_string());
        }
        if let Some(endpoint) = old.remove("jvs_endpoint") {
            set("endpoint", endpoint);
        }
    } else {
        for key in ["vid", "pid"] {
            if let Some(value) = old.get(key) {
                set(key, format!("{:04X}", hex_u16(key, value)?));
            }
        }
    }
    values.insert("controller_driver".to_string(), Some(driver.to_string()));
    Ok(())
}

fn jvs_player(value: &str) -> Result<&'static str, String> {
    match value.to_ascii_lowercase().as_str() {
        "p1" | "1" => Ok("p1"),
        "p2" | "2" => Ok("p2"),
        _ => Err(format!("unknown JVS player profile '{value}'")),
    }
}

fn hex_u16(key: &str, value: &str) -> Result<u16, String> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u16::from_str_radix(digits, 16).map_err(|_| {
        format!("invalid [{CONTROLLER_SECTION}] {key}='{value}': expected 16-bit hexadecimal")
    })
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gui/main_window/domain/config/migration/steps/schema_2__tests.rs"]
mod tests;
