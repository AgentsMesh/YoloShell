use super::*;

const TEST_CAPABILITY: &str = "pty-object-scope";
const TEST_TTY_PATH: &str = "/dev/null";

#[test]
fn pty_object_lease_is_required_only_for_macos_pty_scope() {
    let pty = PtySessionProof::capture(TEST_TTY_PATH.into(), TEST_CAPABILITY.to_owned()).unwrap();
    let discovery = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();

    assert_eq!(pty.requires_pty_object_lease(), cfg!(target_os = "macos"));
    assert_eq!(
        requires_pty_object_lease(Some(&pty)),
        cfg!(target_os = "macos")
    );
    assert!(!discovery.requires_pty_object_lease());
    assert!(!requires_pty_object_lease(Some(&discovery)));
}

#[test]
fn missing_proof_explicitly_requires_no_pty_object_lease() {
    assert!(!requires_pty_object_lease(None));
}

#[test]
fn tree_without_a_proof_requires_no_pty_object_lease() {
    let root = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let tree = crate::process::CapturedProcessTree::capture(root).unwrap();

    assert!(!tree.requires_pty_object_lease());
}
