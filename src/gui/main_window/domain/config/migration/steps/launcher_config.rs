use configparser::ini::Ini;

use super::super::chain::Findings;
use crate::gui::common::hash::fnv1a_64;

pub(in super::super) const FILE_NAME: &str = "launcherConfig.ini";

pub(in super::super) const VERSION: u32 = 1;

const CONTROLLER_SECTION: &str = "controller_screen";
const SCHEMA_2_CONTROLLER_SECTION: &str = "game_launcher_screen";

const PREPARED_KINDS: [u64; 2] = [0x44f8_2cb6_e2f5_ee35, 0xc642_a624_50e9_a75a];

pub(in super::super) fn upgrade(document: &mut Ini, findings: &mut Findings) -> Result<(), String> {
    let sections = document.get_mut_map();
    sections.remove(SCHEMA_2_CONTROLLER_SECTION);
    if let Some(controller) = sections.remove(CONTROLLER_SECTION) {
        sections.insert(SCHEMA_2_CONTROLLER_SECTION.to_string(), controller);
    }
    let mut prepared_games = Vec::new();
    for (section, values) in sections.iter_mut() {
        let Some(id) = section.strip_prefix("game.") else {
            continue;
        };
        values.retain(|key, _| key != "selected_target" && !key.starts_with("plugin_"));
        let target = values
            .remove("exe_path")
            .flatten()
            .filter(|target| !target.trim().is_empty())
            .ok_or_else(|| format!("missing or empty exe_path in [{section}]"))?;
        values.insert("selected_target".to_string(), Some(target));
        let kind = values
            .remove("kind")
            .flatten()
            .filter(|kind| !kind.trim().is_empty())
            .ok_or_else(|| format!("missing or empty kind in [{section}]"))?;
        let word = kind.trim().to_ascii_lowercase();
        if PREPARED_KINDS.contains(&fnv1a_64(word.as_bytes())) {
            let label = values.get("label").cloned().flatten().unwrap_or_default();
            prepared_games.push((
                id.parse::<u64>().unwrap_or(u64::MAX),
                label.trim().to_string(),
            ));
        } else if word != "other" {
            return Err(format!("unknown game kind '{kind}' in [{section}]"));
        }
    }
    prepared_games.sort();
    findings
        .plugin_needed
        .extend(prepared_games.into_iter().map(|(_, label)| label));
    Ok(())
}

#[cfg(test)]
#[path = "../../../../../../../tests/unit/gui/main_window/domain/config/migration/steps/launcher_config__tests.rs"]
mod tests;
