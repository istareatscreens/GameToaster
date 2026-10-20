mod normalize;
mod plan;
mod rules;
mod verify;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use plan::{build_plan, build_plan_preview, rebind_destination_leaves, revalidate_plan};

use crate::support::paths::paths_equal;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZeroMatchChoice {
    CopyToGameFolder,
    NoAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryFallbackChoice {
    SearchDirectory,
    NoAction,
}

#[derive(Clone, Debug, Default)]
pub struct ResolutionChoices {
    pub zero_match: BTreeMap<usize, ZeroMatchChoice>,
    pub directory_fallback: BTreeMap<usize, DirectoryFallbackChoice>,
    pub launcher_destination: Option<PathBuf>,
    pub watch_destination: Option<PathBuf>,
    pub external_grants: Vec<ExternalGrant>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExternalGrant {
    pub plugin_id: String,
    pub game_id: u64,
    pub revision_sha256: String,
    pub destination_root: PathBuf,
    pub root_identity: Option<FileIdentity>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FileIdentity {
    pub volume_serial: u32,
    pub file_index: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedFileKind {
    Config,
    Plugin,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlannedFile {
    pub rule_index: usize,
    pub kind: ManagedFileKind,
    pub source_relative: String,
    pub source: PathBuf,
    pub source_sha256: String,
    pub source_size: u64,
    pub destination: PathBuf,
    pub destination_identity: Option<FileIdentity>,
    pub verified_parent: PathBuf,
    pub parent_identity: FileIdentity,
    pub destination_existed: bool,
    pub outside_game_folder: bool,
    pub add_to_config_menu: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DeploymentPlan {
    pub plan_version: u32,
    pub plugin_id: String,
    pub revision_sha256: String,
    pub game_id: u64,
    pub game_folder: PathBuf,
    pub selected_target: PathBuf,
    pub resolved_launcher: Option<PathBuf>,
    pub resolved_watch_target: Option<PathBuf>,
    pub files: Vec<PlannedFile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmbiguityKind {
    Launcher,
    WatchTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionAmbiguity {
    pub kind: AmbiguityKind,
    pub candidates: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct PlanPreview {
    pub plan: DeploymentPlan,
    pub required_external_grants: Vec<ExternalGrant>,
    pub ambiguities: Vec<ResolutionAmbiguity>,
}

impl PlanPreview {
    pub fn is_ready(&self) -> bool {
        self.required_external_grants.is_empty() && self.ambiguities.is_empty()
    }

    pub fn approve(
        mut self,
        granted_external_roots: &[PathBuf],
        launcher_destination: Option<&Path>,
        watch_destination: Option<&Path>,
    ) -> Result<(DeploymentPlan, Vec<ExternalGrant>), String> {
        for grant in &self.required_external_grants {
            if !granted_external_roots
                .iter()
                .any(|root| paths_equal(root, &grant.destination_root))
            {
                return Err(format!(
                    "external destination was not granted: {}",
                    grant.destination_root.display()
                ));
            }
        }
        for ambiguity in &self.ambiguities {
            let (label, selected, destination) = match ambiguity.kind {
                AmbiguityKind::Launcher => (
                    "gameLauncher",
                    launcher_destination,
                    &mut self.plan.resolved_launcher,
                ),
                AmbiguityKind::WatchTarget => (
                    "executableWatch",
                    watch_destination,
                    &mut self.plan.resolved_watch_target,
                ),
            };
            let selected = selected
                .ok_or_else(|| format!("{label} requires an explicit candidate selection"))?;
            let candidate = ambiguity
                .candidates
                .iter()
                .find(|candidate| paths_equal(candidate, selected))
                .cloned()
                .ok_or_else(|| format!("{label} selection is not a previewed candidate"))?;
            *destination = Some(candidate);
        }
        Ok((self.plan, self.required_external_grants))
    }
}

#[derive(Clone, Debug)]
pub struct PathPolicy {
    pub application_root: PathBuf,
    pub catalog_root: PathBuf,
    pub state_root: PathBuf,
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/paths/mod__tests.rs"]
mod tests;
