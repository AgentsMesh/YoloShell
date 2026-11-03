use super::*;
use crate::paths::directory::PrivateDirectory;

const NAMESPACE: &str = "guard versioned read fixture";
const VALUE: u64 = 71;
const MALFORMED_BYTES: &[u8] = b"malformed json";

#[test]
fn guarded_optional_versions_distinguish_absence_from_invalid_or_forbidden_state() {
    let root = tempfile::tempdir().unwrap();
    PrivateDirectory::ensure(root.path()).unwrap();
    let paths = Paths::new(root.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let path = paths.runtime_record(terminal_id);

    assert!(
        guard
            .read_optional_with_versions::<u64>(&path)
            .unwrap()
            .is_none()
    );
    assert!(guard.durable_identity.get().is_none());
    for forbidden in [
        paths.reboot_migration_intent(terminal_id),
        paths.reboot_preparation_authority(terminal_id),
        session.join(SESSION_LIFECYCLE_LOCK_FILE_NAME),
        paths.runtime_record(Uuid::new_v4()),
    ] {
        assert!(
            guard
                .read_optional_with_versions::<u64>(&forbidden)
                .is_err()
        );
    }
    namespace.write_json(&path, &VALUE).unwrap();
    let (value, live, version) = guard
        .read_optional_with_versions::<u64>(&path)
        .unwrap()
        .unwrap();
    assert_eq!(value, VALUE);
    let opened = namespace
        .file(&path)
        .unwrap()
        .open_read(crate::paths::MAX_DURABLE_JSON_BYTES)
        .unwrap();
    assert_eq!(live, opened.identity());
    assert_eq!(
        version,
        opened
            .durable_version(&path, crate::paths::MAX_DURABLE_JSON_BYTES)
            .unwrap()
    );

    std::fs::write(&path, MALFORMED_BYTES).unwrap();
    assert!(guard.read_optional_with_versions::<u64>(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), MALFORMED_BYTES);
}
