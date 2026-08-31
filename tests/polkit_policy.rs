const POLICY: &str = include_str!("../packaging/io.github.atvv_bridge.button-mapping.policy");

#[test]
fn modify_policy_is_bound_to_the_fixed_helper_without_retained_authorization() {
    assert_eq!(POLICY.matches("<action id=").count(), 1);
    assert!(POLICY.contains(
        r#"<annotate key="org.freedesktop.policykit.exec.path">/usr/libexec/atvv-bridge/atvv-button-mapping-helper</annotate>"#
    ));
    assert!(POLICY.contains("<allow_active>auth_admin</allow_active>"));
    assert!(!POLICY.contains("auth_admin_keep"));
    assert!(!POLICY.contains("org.freedesktop.policykit.imply"));
}
