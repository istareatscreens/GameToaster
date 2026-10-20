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
