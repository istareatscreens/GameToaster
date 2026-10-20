use configparser::ini::Ini;

use super::super::{parse_current, LAUNCHER_SECTION, SCHEMA_VERSION};
use super::steps::{launcher_config, schema_2};
use crate::launcher_gui::model::LauncherConfig;

#[derive(Debug, Default)]
pub(super) struct Findings {
    pub plugin_needed: Vec<String>,
}

pub(super) struct Step {
    pub from: u32,
    pub upgrade: fn(&mut Ini, &mut Findings) -> Result<(), String>,
}

pub(super) const STEPS: [Step; 2] = [
    Step {
        from: launcher_config::VERSION,
        upgrade: launcher_config::upgrade,
    },
    Step {
        from: schema_2::VERSION,
        upgrade: schema_2::upgrade,
    },
];

pub(super) struct Upgraded {
    pub config: LauncherConfig,
    pub plugin_needed: Vec<String>,
}

pub(super) fn upgrade(mut document: Ini, version: u32) -> Result<Upgraded, String> {
    let mut findings = Findings::default();
    let mut at = version;
    while at < SCHEMA_VERSION {
        let step = STEPS
            .iter()
            .find(|step| step.from == at)
            .ok_or_else(|| format!("no step upgrades version {at}"))?;
        (step.upgrade)(&mut document, &mut findings)
            .map_err(|error| format!("upgrading version {at}: {error}"))?;
        at += 1;
        document.setstr(LAUNCHER_SECTION, "schema_version", Some(&at.to_string()));
    }
    let config = parse_current(&document)?;
    Ok(Upgraded {
        config,
        plugin_needed: findings.plugin_needed,
    })
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/domain/config/migration/chain__tests.rs"]
mod tests;
