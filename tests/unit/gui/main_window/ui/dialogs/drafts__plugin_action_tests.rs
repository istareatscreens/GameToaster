use super::super::harness::plugin_action_preview as preview;
use super::*;
use crate::plugin::deployment::ModificationDecision;

#[test]
fn approval_requires_previewed_choices_grants_and_modification_decisions() {
    let mut draft = PluginActionDraft::new(preview());
    assert!(!draft.is_complete());
    assert!(draft.confirmation_event().is_none());

    draft.approval.launcher_destination = Some(r"C:\Games\not-previewed.exe".into());
    draft.set_external_granted(r"c:\shared".into(), true);
    let modification_key =
        crate::plugin::deployment::decision_key(std::path::Path::new(r"C:\Games\config.ini"));
    draft
        .approval
        .modification_decisions
        .insert(modification_key, ModificationDecision::BackupAndOverwrite);
    assert!(!draft.is_complete());

    draft.approval.launcher_destination = Some(r"c:\games\RUNNER.exe".into());
    assert!(draft.is_complete());
    match draft.confirmation_event().unwrap() {
        super::super::DialogEvent::ConfirmPluginAction { token, approval } => {
            assert_eq!(token, draft.preview.token);
            assert_eq!(approval.modification_decisions.len(), 1);
        }
        _ => panic!("completion emitted the wrong dialog event"),
    }
}

#[test]
fn cancellation_carries_only_the_exact_preview_token() {
    let draft = PluginActionDraft::new(preview());
    match draft.cancellation_event() {
        super::super::DialogEvent::CancelPluginAction(token) => {
            assert_eq!(token, draft.preview.token)
        }
        _ => panic!("cancellation emitted the wrong dialog event"),
    }
}

#[test]
fn a_replaced_plugin_file_needs_no_answer_and_a_given_up_file_needs_one() {
    use crate::plugin::deployment::{ModificationPurpose, UserModification};
    let complete_otherwise = |draft: &mut PluginActionDraft| {
        draft.approval.launcher_destination = Some(r"c:\games\RUNNER.exe".into());
        draft.set_external_granted(r"c:\shared".into(), true);
    };

    let mut draft = PluginActionDraft::new(preview());
    assert!(draft
        .preview
        .modifications
        .iter()
        .all(|modification| modification.purpose == ModificationPurpose::ReplaceWithPlugin));
    complete_otherwise(&mut draft);
    assert!(draft.is_complete(), "a replaced file needs no answer");
    match draft.confirmation_event().unwrap() {
        super::super::DialogEvent::ConfirmPluginAction { approval, .. } => {
            assert!(approval.modification_decisions.is_empty())
        }
        _ => panic!("completion emitted the wrong dialog event"),
    }

    let retired = std::path::PathBuf::from(r"C:\Games\retired.dll");
    let mut given_up = preview();
    given_up.modifications.push(UserModification {
        destination: retired.clone(),
        purpose: ModificationPurpose::RestoreOriginal,
        last_deployed_sha256: "e".repeat(64),
        current_sha256: None,
    });
    let mut draft = PluginActionDraft::new(given_up);
    complete_otherwise(&mut draft);
    assert!(!draft.is_complete());
    draft.approval.modification_decisions.insert(
        crate::plugin::deployment::decision_key(&retired),
        ModificationDecision::KeepCurrent,
    );
    assert!(draft.is_complete());
}
