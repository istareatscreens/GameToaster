use super::*;

#[test]
fn a_portable_executable_keeps_its_state_beside_itself() {
    let executable = std::env::current_exe().unwrap();
    assert_eq!(
        state_root(InstallKind::Portable).unwrap(),
        executable.parent().unwrap()
    );
}
