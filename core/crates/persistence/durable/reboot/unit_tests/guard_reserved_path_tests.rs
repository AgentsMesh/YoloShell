use super::guard::SessionDurableMutationGuard;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const NAMESPACE_LABEL: &str = "durable reboot reserved path test";
const REPLACEMENT_VALUE: u8 = 1;

#[test]
fn managed_writer_cannot_replace_the_live_session_lifecycle_lock() {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let lifecycle = paths.session_lifecycle_lock_path(terminal_id);

    assert!(
        guard
            .write_json_expected_with_commit_fence(&lifecycle, None, &REPLACEMENT_VALUE,)
            .is_err()
    );
    assert!(
        paths
            .try_acquire_session_lifecycle_lock(terminal_id)
            .unwrap()
            .is_none()
    );
    drop(guard);
    assert!(
        paths
            .try_acquire_session_lifecycle_lock(terminal_id)
            .unwrap()
            .is_some()
    );
}
