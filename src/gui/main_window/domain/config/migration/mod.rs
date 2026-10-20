mod backup;
mod chain;
mod outcome;
mod steps;

use std::cmp::Ordering;
use std::path::Path;

use super::{parse_current, read_document, save_new, schema_version, FILE_NAME, SCHEMA_VERSION};
use crate::launcher_gui::model::LauncherConfig;

pub use outcome::{Notice, ResetReason};

pub struct Opened {
    pub config: LauncherConfig,
    pub notice: Option<Notice>,
}

pub fn open(state_root: &Path) -> Result<Opened, String> {
    let current = state_root.join(FILE_NAME);
    if current.is_file() {
        return open_current(&current);
    }
    let previous = state_root.join(steps::launcher_config::FILE_NAME);
    if previous.is_file() {
        return open_previous(&current, &previous);
    }
    Ok(Opened {
        config: LauncherConfig::default(),
        notice: None,
    })
}

fn open_current(path: &Path) -> Result<Opened, String> {
    let document = match read_document(path) {
        Ok(document) => document,
        Err(error) => return reset(path, ResetReason::Unreadable(error)),
    };
    let Some(version) = schema_version(&document) else {
        return reset(path, ResetReason::UnknownVersion);
    };
    match version.cmp(&SCHEMA_VERSION) {
        Ordering::Equal => Ok(Opened {
            config: parse_current(&document)?,
            notice: None,
        }),
        Ordering::Greater => reset(path, ResetReason::Newer(version)),
        Ordering::Less => match chain::upgrade(document, version) {
            Ok(upgraded) => {
                let kept = backup::keep(path, Some(&format!("schema-{version}")))?;
                save_new(path, &upgraded.config)?;
                Ok(Opened {
                    config: upgraded.config,
                    notice: Some(Notice::Upgraded {
                        kept,
                        plugin_needed: upgraded.plugin_needed,
                    }),
                })
            }
            Err(error) => reset(path, ResetReason::NotUpgradable(error)),
        },
    }
}

fn open_previous(current: &Path, previous: &Path) -> Result<Opened, String> {
    let upgraded = read_document(previous)
        .map_err(ResetReason::Unreadable)
        .and_then(|document| {
            chain::upgrade(document, steps::launcher_config::VERSION)
                .map_err(ResetReason::NotUpgradable)
        });
    let (config, notice) = match upgraded {
        Ok(upgraded) => (
            upgraded.config,
            Notice::Upgraded {
                kept: previous.to_path_buf(),
                plugin_needed: upgraded.plugin_needed,
            },
        ),
        Err(reason) => (
            LauncherConfig::default(),
            Notice::Reset {
                reason,
                kept: previous.to_path_buf(),
            },
        ),
    };
    save_new(current, &config)?;
    Ok(Opened {
        config,
        notice: Some(notice),
    })
}

fn reset(path: &Path, reason: ResetReason) -> Result<Opened, String> {
    let kept = backup::keep(path, None)?;
    let config = LauncherConfig::default();
    save_new(path, &config)?;
    Ok(Opened {
        config,
        notice: Some(Notice::Reset { reason, kept }),
    })
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/domain/config/migration/mod__fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "../../../../../../tests/unit/gui/main_window/domain/config/migration/mod__tests.rs"]
mod tests;
