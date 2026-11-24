#[cfg(target_os = "macos")]
async fn assert_promoted_custody_fenced_for_retry(
    fixture: &Fixture,
    wire: &SuccessorAdoptionWire,
    offer: &crate::pty_custody::CustodyOffer,
    predecessor_generation: u64,
) {
    let successor_generation = wire.request.controller_generation;
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        successor_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        successor_generation
    );
    let memory = lock_recover(&fixture.state.record).clone();
    let durable_runtime: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(durable_runtime, memory);
    assert_eq!(durable_runtime.controller_generation, successor_generation);

    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .expect("promoted custody must remain recoverable");
    assert_eq!(identity.lease_id(), offer.lease_id);
    assert_eq!(identity.offer_id(), offer.offer_id);
    assert_eq!(identity.generation(), successor_generation);
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(identity)
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .release_identity_for_test()
            .is_none()
    );
    let durable_custody: crate::pty_custody::CustodyDurableRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.pty_custody(fixture.terminal_id()))
        .unwrap();
    assert!(identity.matches_durable(durable_custody.armed_lease().unwrap()));
    assert!(
        fixture
            .state
            .admit_controller_mutation(AuthenticatedController::new(predecessor_generation))
            .await
            .is_err()
    );
}
