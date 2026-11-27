use crate::worker::tests::*;
const RECONCILED_CONTROLLER_GENERATION: u64 = FIXTURE_CONTROLLER_GENERATION + 1;

#[tokio::test]
async fn indeterminate_controller_write_reconciles_only_the_exact_record() {
    let fixture = Fixture::new();
    let mut expected = lock_recover(&fixture.state.record).clone();
    expected.controller_generation = RECONCILED_CONTROLLER_GENERATION;
    fixture
        .state
        .control_test_hooks
        .make_next_controller_persist_indeterminate();

    let controller = fixture
        .state
        .adopt_controller(RECONCILED_CONTROLLER_GENERATION)
        .await
        .unwrap();

    let persisted: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(persisted, expected);
    assert_eq!(*lock_recover(&fixture.state.record), expected);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        RECONCILED_CONTROLLER_GENERATION
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        RECONCILED_CONTROLLER_GENERATION
    );
    assert!(fixture.state.validate_controller(controller).is_ok());
}

#[tokio::test]
async fn successor_controller_replaces_the_predecessor_writer_lease() {
    const PREDECESSOR_HOLDER: &str = "predecessor-writer";
    const SUCCESSOR_HOLDER: &str = "successor-writer";
    let fixture = Fixture::new();
    let predecessor = fixture.lease_request(PREDECESSOR_HOLDER);
    fixture
        .state
        .acquire_lease(fixture.controller(), &predecessor, false)
        .await
        .unwrap();

    let controller = fixture
        .state
        .adopt_controller(RECONCILED_CONTROLLER_GENERATION)
        .await
        .unwrap();
    let successor = fixture.lease_request(SUCCESSOR_HOLDER);
    let grant = fixture
        .state
        .acquire_lease(controller, &successor, false)
        .await
        .unwrap();

    assert_eq!(grant.lease.holder_id, SUCCESSOR_HOLDER);
    assert_ne!(grant.writer_capability, predecessor.writer_capability);
}
