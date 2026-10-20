use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    Upgraded {
        kept: PathBuf,
        plugin_needed: Vec<String>,
    },

    Reset {
        reason: ResetReason,
        kept: PathBuf,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResetReason {
    UnknownVersion,

    Newer(u32),

    Unreadable(String),

    NotUpgradable(String),
}
