use super::*;

const SESSION_NAMESPACE_LABEL: &str = "durable reboot settlement test session";

#[test]
fn absent_intent_does_not_contend_for_the_session_lifecycle_lock() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(SESSION_NAMESPACE_LABEL, &session)]).unwrap();
    let _lifecycle = paths
        .try_acquire_session_lifecycle_lock(terminal_id)
        .unwrap()
        .unwrap();

    assert!(!settle_pending_session(&paths, &namespace, terminal_id).unwrap());
}

#[test]
fn retry_converges_after_one_participant_cleanup() {
    let fixture = super::super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(SESSION_NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let loaded = super::super::publish_intent(&paths, &guard, &fixture.intent).unwrap();
    super::super::install::install_prerequisites(&paths, &guard, &loaded).unwrap();
    super::super::install::install_runtime(&paths, &guard, &loaded).unwrap();

    let checkpoint = paths.worker_startup_checkpoint(terminal_id);
    let super::super::OptionalParticipant::Replace(required) = &fixture.intent.startup_checkpoint
    else {
        unreachable!();
    };
    super::super::staging::cleanup(
        &guard,
        &checkpoint,
        fixture.intent.transaction_id,
        super::super::constants::STARTUP_CHECKPOINT_PARTICIPANT,
        required.source_version,
    )
    .unwrap();
    super::super::receipt::clear(
        &guard,
        &checkpoint,
        fixture.intent.transaction_id,
        super::super::constants::STARTUP_CHECKPOINT_PARTICIPANT,
        required.prepared_receipt_version,
        || super::super::intent_io::verify_exact(&paths, &guard, &loaded),
    )
    .unwrap();
    drop(guard);

    assert!(settle_pending_session(&paths, &namespace, terminal_id).unwrap());
    assert!(
        namespace
            .file_is_missing(&paths.reboot_migration_intent(terminal_id))
            .unwrap()
    );
}
