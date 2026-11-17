use crate::worker::tests::*;
#[cfg(target_os = "macos")]
fn install_post_publish_manifest_archive(fixture: &Fixture) {
    let state = Arc::clone(&fixture.state);
    let terminal_id = fixture.terminal_id();
    fixture
        .state
        .control_test_hooks
        .after_next_custody_runtime_publish(move || {
            let path = state.paths.manifest(terminal_id);
            let mut manifest: crate::model::TerminalManifest =
                state.namespace.read_json(&path).unwrap();
            manifest.archived = true;
            state.namespace.write_json(&path, &manifest).unwrap();
        });
}

#[cfg(target_os = "macos")]
fn install_post_publish_custody_replacement(fixture: &Fixture) {
    let state = Arc::clone(&fixture.state);
    let terminal_id = fixture.terminal_id();
    fixture
        .state
        .control_test_hooks
        .after_next_custody_runtime_publish(move || {
            let path = state.paths.pty_custody(terminal_id);
            let custody: crate::pty_custody::CustodyDurableRecord =
                state.namespace.read_json(&path).unwrap();
            state.namespace.write_json(&path, &custody).unwrap();
        });
}

#[cfg(target_os = "macos")]
pub(super) async fn assert_indeterminate_promotion_fails_closed(
    fixture: &Fixture,
    install_boundary: impl FnOnce(&Fixture),
) {
    let previous = lock_recover(&fixture.state.record).clone();
    let previous_authority = fixture.state.secondary_custody_authority.active_for_test();
    install_boundary(fixture);
    fixture
        .state
        .control_test_hooks
        .make_next_controller_persist_indeterminate();
    let mut wire = SuccessorAdoptionWire::install(fixture);
    let successor_generation = wire.request.controller_generation;
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;

    let mut peer = wire.take_peer();
    acknowledge_exact_worker_release(&mut peer, &offer).await;
    await_adoption_failure(adoption).await;
    let durable: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(durable.controller_generation, successor_generation);
    assert_eq!(*lock_recover(&fixture.state.record), previous);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        previous.controller_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        previous.controller_generation
    );
    assert_ne!(
        fixture
            .state
            .secondary_custody_authority
            .active_for_test()
            .map(|identity| identity.generation()),
        Some(successor_generation)
    );
    assert!(previous_authority.is_some());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn archived_manifest_fences_indeterminate_runtime_promotion_reconciliation() {
    let fixture = Fixture::new();
    assert_indeterminate_promotion_fails_closed(&fixture, install_post_publish_manifest_archive)
        .await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn post_cas_custody_replacement_fences_indeterminate_runtime_reconciliation() {
    let fixture = Fixture::new();
    assert_indeterminate_promotion_fails_closed(&fixture, install_post_publish_custody_replacement)
        .await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn different_guardian_for_exact_candidate_fences_worker_promotion() {
    let fixture = Fixture::new();
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    replace_candidate_guardian(&fixture, &wire, &offer);
    wire.send_ready(&offer).await;

    let mut peer = wire.take_peer();
    acknowledge_exact_worker_release(&mut peer, &offer).await;
    await_adoption_failure(adoption).await;
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        predecessor_generation
    );
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(predecessor)
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .release_identity_for_test()
            .is_none()
    );
}

#[cfg(target_os = "macos")]
const CANDIDATE_NOT_ARMED_DETAIL: &str = "successor custody candidate is not armed";

#[cfg(target_os = "macos")]
fn replace_candidate_guardian(
    fixture: &Fixture,
    wire: &SuccessorAdoptionWire,
    offer: &crate::pty_custody::CustodyOffer,
) {
    let path = fixture.state.paths.pty_custody(fixture.terminal_id());
    let mut durable: crate::pty_custody::CustodyDurableRecord =
        fixture.state.namespace.read_json(&path).unwrap();
    let replacement = crate::pty_custody::CustodyGuardianBinding::new(
        uuid::Uuid::new_v4(),
        wire.request.holder(),
        &wire.request,
        offer,
    )
    .unwrap();
    let crate::pty_custody::CustodyDurableState::Armed { guardian, .. } = &mut durable.state else {
        panic!("{CANDIDATE_NOT_ARMED_DETAIL}");
    };
    let original = guardian.replace(replacement).unwrap();
    assert_ne!(original, replacement);
    assert!(durable.is_valid());
    fixture.state.namespace.write_json(&path, &durable).unwrap();
}
