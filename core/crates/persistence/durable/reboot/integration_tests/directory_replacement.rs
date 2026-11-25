use super::guard::SessionDurableMutationGuard;
use super::validation::test_fixture::Fixture;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const NAMESPACE_LABEL: &str = "reboot replacement adversary";
const HELD_DIRECTORY: &str = "retained-original-session";

#[test]
fn copied_session_at_the_same_path_cannot_adopt_a_persisted_migration() {
    let fixture = Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    {
        let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
        let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
        super::publish_intent(&paths, &guard, &fixture.intent).unwrap();
    }
    let held = fixture._root.path().join(HELD_DIRECTORY);
    std::fs::rename(&session, &held).unwrap();
    super::copied_directory::copy_directory(&held, &session).unwrap();
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    assert!(super::settle_pending_session_locked(&paths, &guard).is_err());
    assert!(
        namespace
            .file_is_missing(&paths.reboot_migration_intent(terminal_id))
            .is_ok_and(|missing| !missing)
    );
    let runtime: crate::model::RuntimeRecord = namespace
        .read_json(&paths.runtime_record(terminal_id))
        .unwrap();
    assert_eq!(runtime.boot_id, fixture.intent.source_boot_id);
}
