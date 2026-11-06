use super::resume_identity::owner_allowed;

#[test]
fn cwd_identity_accepts_shared_ownership_but_private_paths_require_current_user() {
    assert!(owner_allowed(0, 501, false));
    assert!(owner_allowed(501, 501, false));
    assert!(owner_allowed(501, 501, true));
    assert!(!owner_allowed(0, 501, true));
}
