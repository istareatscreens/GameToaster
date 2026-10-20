use super::*;

#[test]
fn the_descriptor_is_built_for_the_current_user_and_is_not_inheritable() {
    let descriptor = SecurityDescriptor::current_user_only().unwrap();
    let attributes = descriptor.attributes();
    assert!(!attributes.lpSecurityDescriptor.is_null());
    assert!(!attributes.bInheritHandle.as_bool());
    assert_eq!(
        attributes.nLength as usize,
        std::mem::size_of::<SECURITY_ATTRIBUTES>()
    );
}
