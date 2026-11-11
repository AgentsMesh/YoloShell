use super::*;
use crate::paths::directory::PrivateDirectory;

const NAMESPACE: &str = "cached durable directory fixture";
const SAVED_SESSION: &str = "displaced-session";

#[test]
fn cached_guard_keeps_rejecting_directory_replacement_and_never_seeds_a_new_guard() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let paths = Paths::new(root.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    assert!(guard.durable_identity.get().is_none());
    let original = guard.session_durable_identity().unwrap();
    assert_eq!(guard.session_durable_identity().unwrap(), original);
    assert_eq!(guard.durable_identity.get(), Some(&original));

    std::fs::rename(&session, root.path().join(SAVED_SESSION)).unwrap();
    PrivateDirectory::ensure(&session).unwrap();
    assert!(guard.session_durable_identity().is_err());
    assert!(guard.verify_session_directory().is_err());
    drop(guard);

    let namespace = NamespaceLease::capture(&[(NAMESPACE, &session)]).unwrap();
    let fresh = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    assert!(fresh.durable_identity.get().is_none());
    assert_ne!(fresh.session_durable_identity().unwrap(), original);
}

#[test]
fn failed_first_live_verification_does_not_cache_durable_authority() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let paths = Paths::new(root.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    std::fs::rename(&session, root.path().join(SAVED_SESSION)).unwrap();
    PrivateDirectory::ensure(&session).unwrap();

    assert!(guard.session_durable_identity().is_err());
    assert!(guard.durable_identity.get().is_none());
}
