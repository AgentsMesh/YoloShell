use super::super::guard::SessionDurableMutationGuard;
use super::super::validation::test_fixture::Fixture;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const NAMESPACE_LABEL: &str = "durable reboot install identity test";

#[test]
fn installed_retry_rejects_same_payload_on_a_replacement_inode() {
    let fixture = Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let loaded = super::super::publish_intent(&paths, &guard, &fixture.intent).unwrap();
    super::super::install::install_prerequisites(&paths, &guard, &loaded).unwrap();
    super::super::install::install_runtime(&paths, &guard, &loaded).unwrap();

    namespace
        .write_json(
            &paths.runtime_record(terminal_id),
            &fixture.intent.runtime.target_payload,
        )
        .unwrap();

    assert!(super::super::install::install_runtime(&paths, &guard, &loaded).is_err());
}
