use super::super::harness;
use super::super::Dialog;
use super::*;
use crate::plugin::paths::ResolutionAmbiguity;
use crate::plugin::workflow::PluginActionPreview;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn preview(ambiguous: bool) -> PluginActionPreview {
    PluginActionPreview {
        token: TOKEN.into(),
        action: WorkflowAction::Install,
        plugin_id: "org.example.plugin".into(),
        plugin_version: semver::Version::parse("1.2.3").unwrap(),
        plugin_md5: "a".repeat(32),
        plugin_sha256: "b".repeat(64),
        label: "Example".into(),
        description: "Fixture".into(),
        selected_target: r"C:\Games\game.exe".into(),
        resolved_launcher: None,
        resolved_watch_target: None,
        files: Vec::new(),
        install_scripts: Vec::new(),
        required_external_grants: Vec::new(),
        ambiguities: if ambiguous {
            vec![ResolutionAmbiguity {
                kind: AmbiguityKind::Launcher,
                candidates: vec![r"C:\Games\runner.exe".into()],
            }]
        } else {
            Vec::new()
        },
        modifications: Vec::new(),
        assignment_state: None,
        latest_transaction_state: None,
        transaction_count: 0,
    }
}

#[test]
fn review_cancels_with_the_plans_own_event_and_approves_only_a_complete_plan() {
    let mut shown = harness::show(Dialog::PluginAction(PluginActionDraft::new(preview(true))));
    assert_eq!(shown.actions().len(), 2);
    assert!(
        matches!(shown.click_action(1), DialogEvent::None),
        "Approve is disabled until the plan is complete"
    );
    match shown.click_action(0) {
        DialogEvent::CancelPluginAction(token) => assert_eq!(token, TOKEN),
        other => panic!("{other:?}"),
    }

    let mut shown = harness::show(Dialog::PluginAction(PluginActionDraft::new(preview(false))));
    match shown.click_action(1) {
        DialogEvent::ConfirmPluginAction { token, .. } => assert_eq!(token, TOKEN),
        other => panic!("{other:?}"),
    }
}

fn painted(shown: &harness::Shown) -> Vec<String> {
    crate::gui::common::harness::painted_texts(&shown.output)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

#[test]
fn a_replaced_plugin_file_is_shown_as_backed_up_and_replaced_and_asks_nothing() {
    use crate::plugin::deployment::{ModificationPurpose, UserModification};
    let _locale = crate::gui::common::harness::locale("en");
    let mut plan = preview(false);
    plan.action = WorkflowAction::Update;
    plan.modifications = vec![UserModification {
        destination: r"C:\Games\plugin.dll".into(),
        purpose: ModificationPurpose::ReplaceWithPlugin,
        last_deployed_sha256: "c".repeat(64),
        current_sha256: Some("d".repeat(64)),
    }];
    let mut shown = harness::show(Dialog::PluginAction(PluginActionDraft::new(plan.clone())));
    let texts = painted(&shown);
    for expected in [
        "These plugin files are not the ones GameToaster last deployed. Each one is backed up, then replaced with the file of the version you selected.",
        "A dated backup is taken, then it is replaced",
    ] {
        assert!(texts.iter().any(|text| text == expected), "{expected:?} in {texts:#?}");
    }
    assert!(
        !texts
            .iter()
            .any(|text| text == "Keep my file (leave it unmodified)"),
        "{texts:#?}"
    );
    match shown.click_action(1) {
        DialogEvent::ConfirmPluginAction { token, approval } => {
            assert_eq!(token, TOKEN);
            assert!(approval.modification_decisions.is_empty());
        }
        other => panic!("{other:?}"),
    }

    plan.modifications[0].purpose = ModificationPurpose::RestoreOriginal;
    let mut shown = harness::show(Dialog::PluginAction(PluginActionDraft::new(plan)));
    let texts = painted(&shown);
    assert!(
        texts
            .iter()
            .any(|text| text == "Keep my file (leave it unmodified)"),
        "{texts:#?}"
    );
    assert!(matches!(shown.click_action(1), DialogEvent::None));
}
