use rust_i18n::t;

use super::resolution::{
    assignment_resolution_choices, resolution_choices, selection_resolution_choices,
};
use super::summary::plugin_action_summary;
use super::{PendingPluginAction, PendingPluginKind};
use crate::gui::main_window::app::worker::plugin_backend::PluginBackend;
use crate::launcher_gui::model::{GameEntry, GameId, LauncherConfig, PluginUpdatePolicy};
use crate::plugin::catalog::library::PluginSelection;
use crate::plugin::paths::{build_plan, build_plan_preview, ResolutionChoices};
use crate::plugin::workflow::{PluginActionPreview, WorkflowAction};

pub(in crate::gui::main_window::app::worker) fn preview_add_game_plugin(
    config: LauncherConfig,
    game: GameEntry,
    selection: PluginSelection,
    backend: &Result<PluginBackend, String>,
) -> Result<(PendingPluginAction, PluginActionPreview), String> {
    if config.game(game.id).is_some() {
        return Err(t!("errors.game_exists", id => game.id).to_string());
    }
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    backend.deployment.recover_all()?;
    let revision = backend.catalog.select_exact(
        &selection.plugin_id,
        &selection.version,
        &selection.sha256,
    )?;
    let manifest = read_revision_manifest(&revision)?;
    let choices = selection_resolution_choices(
        &selection,
        manifest.config_files.len() + manifest.plugin_files.len(),
    );
    let plan_preview = build_plan_preview(
        &revision.root,
        &manifest,
        &revision.package_sha256,
        game.id.0,
        &game.selected_target,
        &choices,
        backend.deployment.path_policy(),
    )?;
    let assignment_id = backend.deployment.new_assignment_id()?;
    let token = crate::plugin::workflow::new_preview_token()?;
    let summary = plugin_action_summary(
        token.clone(),
        WorkflowAction::AddGame,
        &revision,
        &manifest,
        &game.selected_target,
        Some(&plan_preview),
        None,
        Vec::new(),
    );
    Ok((
        PendingPluginAction {
            token,
            revision,
            manifest,
            preview: Some(plan_preview),
            choices,
            existing_ledger: None,
            kind: PendingPluginKind::Add {
                config,
                game,
                selection,
                assignment_id,
            },
        },
        summary,
    ))
}

pub(super) fn preview_apply_game_plugin(
    config: LauncherConfig,
    game_id: GameId,
    selection: PluginSelection,
    backend: &Result<PluginBackend, String>,
) -> Result<(PendingPluginAction, PluginActionPreview), String> {
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    backend.deployment.recover_all()?;
    let game_index = config
        .games
        .iter()
        .position(|game| game.id == game_id)
        .ok_or_else(|| t!("errors.game_missing", id => game_id).to_string())?;
    let game = &config.games[game_index];
    let revision = backend.catalog.select_exact(
        &selection.plugin_id,
        &selection.version,
        &selection.sha256,
    )?;
    let manifest = read_revision_manifest(&revision)?;
    let rule_count = manifest.config_files.len() + manifest.plugin_files.len();
    let token = crate::plugin::workflow::new_preview_token()?;

    match game.plugin.as_ref() {
        None => {
            let choices = selection_resolution_choices(&selection, rule_count);
            let preview = build_plan_preview(
                &revision.root,
                &manifest,
                &revision.package_sha256,
                game_id.0,
                &game.selected_target,
                &choices,
                backend.deployment.path_policy(),
            )?;
            let assignment_id = backend.deployment.new_assignment_id()?;
            let summary = plugin_action_summary(
                token.clone(),
                WorkflowAction::Install,
                &revision,
                &manifest,
                &game.selected_target,
                Some(&preview),
                None,
                Vec::new(),
            );
            Ok((
                PendingPluginAction {
                    token,
                    revision,
                    manifest,
                    preview: Some(preview),
                    choices,
                    existing_ledger: None,
                    kind: PendingPluginKind::Install {
                        config,
                        game_index,
                        selection,
                        assignment_id,
                    },
                },
                summary,
            ))
        }
        Some(existing) if existing.plugin_id == selection.plugin_id => {
            let same_revision = existing.revision.version == revision.plugin_version
                && existing
                    .revision
                    .sha256
                    .eq_ignore_ascii_case(&revision.package_sha256);
            let choices = resolution_choices(
                rule_count,
                selection.copy_zero_matches_to_game_folder,
                selection.search_directory_destinations,
                same_revision
                    .then(|| existing.resolved_launcher.clone())
                    .flatten(),
                same_revision
                    .then(|| existing.resolved_watch_target.clone())
                    .flatten(),
                if same_revision {
                    existing.grants.clone()
                } else {
                    Vec::new()
                },
            );
            let preview = build_plan_preview(
                &revision.root,
                &manifest,
                &revision.package_sha256,
                game_id.0,
                &game.selected_target,
                &choices,
                backend.deployment.path_policy(),
            )?;
            let ledger = backend
                .deployment
                .store()
                .load(game_id.0, &existing.assignment_id)?;
            let modifications = backend
                .deployment
                .deployment_modifications(Some(&ledger), &preview.plan)?;
            let summary = plugin_action_summary(
                token.clone(),
                WorkflowAction::Update,
                &revision,
                &manifest,
                &game.selected_target,
                Some(&preview),
                Some(&ledger),
                modifications,
            );
            Ok((
                PendingPluginAction {
                    token,
                    revision,
                    manifest,
                    preview: Some(preview),
                    choices,
                    existing_ledger: Some(ledger),
                    kind: PendingPluginKind::Update {
                        config,
                        game_index,
                        selection,
                    },
                },
                summary,
            ))
        }
        Some(existing) => {
            let old_revision = backend.catalog.select_exact(
                &existing.plugin_id,
                &existing.revision.version,
                &existing.revision.sha256,
            )?;
            let old_manifest = read_revision_manifest(&old_revision)?;
            let old_choices = assignment_resolution_choices(
                existing,
                old_manifest.config_files.len() + old_manifest.plugin_files.len(),
            );
            let old_plan = build_plan(
                &old_revision.root,
                &old_manifest,
                &old_revision.package_sha256,
                game_id.0,
                &game.selected_target,
                &old_choices,
                backend.deployment.path_policy(),
            )?;
            let old_ledger = backend
                .deployment
                .store()
                .load(game_id.0, &existing.assignment_id)?;
            let modifications = backend.deployment.removal_modifications(&old_ledger)?;
            let choices = selection_resolution_choices(&selection, rule_count);
            let preview = build_plan_preview(
                &revision.root,
                &manifest,
                &revision.package_sha256,
                game_id.0,
                &game.selected_target,
                &choices,
                backend.deployment.path_policy(),
            )?;
            let new_assignment_id = backend.deployment.new_assignment_id()?;
            let summary = plugin_action_summary(
                token.clone(),
                WorkflowAction::Change,
                &revision,
                &manifest,
                &game.selected_target,
                Some(&preview),
                Some(&old_ledger),
                modifications,
            );
            Ok((
                PendingPluginAction {
                    token,
                    revision,
                    manifest,
                    preview: Some(preview),
                    choices,
                    existing_ledger: Some(old_ledger),
                    kind: PendingPluginKind::Change {
                        config,
                        game_index,
                        selection,
                        old_plan,
                        old_manifest,
                        new_assignment_id,
                    },
                },
                summary,
            ))
        }
    }
}

pub(super) fn preview_repair_game_plugin(
    config: LauncherConfig,
    game_id: GameId,
    backend: &Result<PluginBackend, String>,
    re_resolve: bool,
) -> Result<(PendingPluginAction, PluginActionPreview), String> {
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    backend.deployment.recover_all()?;
    let game_index = config
        .games
        .iter()
        .position(|game| game.id == game_id)
        .ok_or_else(|| t!("errors.game_missing", id => game_id).to_string())?;
    let game = &config.games[game_index];
    let assignment = game
        .plugin
        .as_ref()
        .ok_or_else(|| t!("errors.no_plugin_assignment").to_string())?;
    let revision = match assignment.update_policy {
        PluginUpdatePolicy::TrackLatest => backend
            .catalog
            .select_latest(&assignment.plugin_id, assignment.allow_prerelease)?
            .ok_or_else(|| {
                t!(
                    "plugins.error.no_eligible_version",
                    plugin => assignment.plugin_id
                )
                .to_string()
            })?,
        PluginUpdatePolicy::Pinned => backend.catalog.select_exact(
            &assignment.plugin_id,
            &assignment.revision.version,
            &assignment.revision.sha256,
        )?,
    };
    let manifest = read_revision_manifest(&revision)?;
    let same_revision = assignment.revision.version == revision.plugin_version
        && assignment
            .revision
            .sha256
            .eq_ignore_ascii_case(&revision.package_sha256);
    let choices = resolution_choices(
        manifest.config_files.len() + manifest.plugin_files.len(),
        assignment.copy_zero_matches_to_game_folder,
        assignment.search_directory_destinations,
        (same_revision && !re_resolve)
            .then(|| assignment.resolved_launcher.clone())
            .flatten(),
        (same_revision && !re_resolve)
            .then(|| assignment.resolved_watch_target.clone())
            .flatten(),
        if same_revision && !re_resolve {
            assignment.grants.clone()
        } else {
            Vec::new()
        },
    );
    let preview = build_plan_preview(
        &revision.root,
        &manifest,
        &revision.package_sha256,
        game_id.0,
        &game.selected_target,
        &choices,
        backend.deployment.path_policy(),
    )?;
    let ledger = backend
        .deployment
        .store()
        .load(game_id.0, &assignment.assignment_id)?;
    let modifications = backend
        .deployment
        .deployment_modifications(Some(&ledger), &preview.plan)?;
    let selection = PluginSelection {
        plugin_id: revision.plugin_id.clone(),
        version: revision.plugin_version.clone(),
        sha256: revision.package_sha256.clone(),
        track_latest: assignment.update_policy.tracks_latest(),
        allow_prerelease: assignment.allow_prerelease,
        copy_zero_matches_to_game_folder: assignment.copy_zero_matches_to_game_folder,
        search_directory_destinations: assignment.search_directory_destinations,
    };
    let token = crate::plugin::workflow::new_preview_token()?;
    let summary = plugin_action_summary(
        token.clone(),
        WorkflowAction::Repair,
        &revision,
        &manifest,
        &game.selected_target,
        Some(&preview),
        Some(&ledger),
        modifications,
    );
    Ok((
        PendingPluginAction {
            token,
            revision,
            manifest,
            preview: Some(preview),
            choices,
            existing_ledger: Some(ledger),
            kind: PendingPluginKind::Repair {
                config,
                game_index,
                selection,
            },
        },
        summary,
    ))
}

pub(super) fn preview_remove_game_plugin(
    config: LauncherConfig,
    game_id: GameId,
    backend: &Result<PluginBackend, String>,
) -> Result<(PendingPluginAction, PluginActionPreview), String> {
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    backend.deployment.recover_all()?;
    let game_index = config
        .games
        .iter()
        .position(|game| game.id == game_id)
        .ok_or_else(|| t!("errors.game_missing", id => game_id).to_string())?;
    let game = &config.games[game_index];
    let assignment = game
        .plugin
        .as_ref()
        .ok_or_else(|| t!("errors.no_plugin_assignment").to_string())?;
    let ledger = backend
        .deployment
        .store()
        .load(game_id.0, &assignment.assignment_id)?;
    let revision = backend.catalog.select_exact(
        &assignment.plugin_id,
        &assignment.revision.version,
        &assignment.revision.sha256,
    )?;
    let manifest = read_revision_manifest(&revision)?;
    let modifications = backend.deployment.removal_modifications(&ledger)?;
    let token = crate::plugin::workflow::new_preview_token()?;
    let summary = plugin_action_summary(
        token.clone(),
        WorkflowAction::Remove,
        &revision,
        &manifest,
        &game.selected_target,
        None,
        Some(&ledger),
        modifications,
    );
    Ok((
        PendingPluginAction {
            token,
            revision,
            manifest,
            preview: None,
            choices: ResolutionChoices::default(),
            existing_ledger: Some(ledger),
            kind: PendingPluginKind::Remove { config, game_index },
        },
        summary,
    ))
}

pub(super) fn read_revision_manifest(
    revision: &crate::plugin::catalog::CatalogRevision,
) -> Result<crate::plugin::manifest::Manifest, String> {
    let path = revision.root.join("manifest.toml");
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        t!("errors.read_failed", path => path.display().to_string(), error => error).to_string()
    })?;
    let manifest = crate::plugin::manifest::Manifest::parse(&raw)?;
    if manifest.plugin_id != revision.plugin_id
        || manifest.plugin_version != revision.plugin_version
    {
        return Err(t!("plugins.error.manifest_identity").to_string());
    }
    Ok(manifest)
}
