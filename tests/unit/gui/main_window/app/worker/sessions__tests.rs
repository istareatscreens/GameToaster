use super::*;

fn watch(policy: CompletionPolicy, job_released: bool, target_exited: bool) -> ActiveWatch {
    ActiveWatch {
        policy,
        job_released,
        target_exited,
        terminal_sent: false,
        _target: None,
        _plugin_activity: None,
        _generic_activity: None,
        plugin_cancel: None,
        runtime_thread: None,
    }
}

#[test]
fn completion_is_authoritative_per_policy_and_never_fabricated() {
    assert!(!watch(CompletionPolicy::Job, false, true).is_terminal());
    assert!(watch(CompletionPolicy::Job, true, false).is_terminal());

    assert!(!watch(CompletionPolicy::EscapedProcess, true, false).is_terminal());
    assert!(watch(CompletionPolicy::EscapedProcess, false, true).is_terminal());

    assert!(!watch(CompletionPolicy::FailedWatch, true, true).is_terminal());

    assert!(!watch(CompletionPolicy::PluginRuntime, true, false).is_terminal());
    assert!(watch(CompletionPolicy::PluginRuntime, false, true).is_terminal());
}
