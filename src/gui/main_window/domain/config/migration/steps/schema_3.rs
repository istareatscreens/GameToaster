use configparser::ini::Ini;

use super::super::chain::Findings;

pub(in super::super) const VERSION: u32 = 3;

const GAME_SECTION_PREFIX: &str = "game.";

pub(in super::super) fn upgrade(document: &mut Ini, _: &mut Findings) -> Result<(), String> {
    for (section, values) in document.get_mut_map().iter_mut() {
        if !section.starts_with(GAME_SECTION_PREFIX) {
            continue;
        }
        let close_mode = values.remove("close_mode").flatten();
        values.remove("pre_start_script");
        let mode = close_mode.as_deref().unwrap_or_default().trim();
        let keeps_close_script = match mode.to_ascii_lowercase().as_str() {
            "script" | "bat" => true,
            "" | "kill" | "kill-exe" | "killexe" => false,
            _ => return Err(format!("unknown close_mode '{mode}' in [{section}]")),
        };
        if !keeps_close_script {
            values.remove("close_script");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gui/main_window/domain/config/migration/steps/schema_3__tests.rs"]
mod tests;
