use crate::worker::tests::*;
const DISCONNECTED_AUTHORITY_DETAIL: &str =
    "disconnected predecessor unexpectedly retained controller authority";
#[cfg(target_os = "macos")]
pub(super) const FENCED_ADOPTION_DETAIL: &str = "fenced predecessor unexpectedly emitted a custody offer";
#[cfg(target_os = "macos")]
const MISMATCH_ADOPTION_DETAIL: &str =
    "durable predecessor mismatch unexpectedly emitted a custody offer";
#[cfg(target_os = "macos")]
pub(super) const FORGED_PREDECESSOR_DIGEST_BYTE: u8 = 0x7d;

#[tokio::test]
async fn disconnected_predecessor_cannot_authenticate_current_controller() {
    let fixture = Fixture::new();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(identity).await;

    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(identity)
    );
    let rejection = rejected_worker_auth(&fixture, identity.generation()).await;
    assert_eq!(
        rejection.message,
        secondary_custody_authority::CUSTODY_UNAVAILABLE_DETAIL,
        "{DISCONNECTED_AUTHORITY_DETAIL}"
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn disconnected_exact_predecessor_accepts_higher_generation_successor() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(predecessor).await;
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    let mut peer = wire.take_peer();

    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    assert!(
        fixture
            .state
            .validate_controller(AuthenticatedController::new(
                wire.request.controller_generation,
            ))
            .is_ok()
    );
    drop(peer);
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn fenced_predecessor_cannot_be_adopted() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    fixture.state.fence_all_custody().await;
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);

    assert_adoption_closed_before_offer(&mut wire, FENCED_ADOPTION_DETAIL).await;
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn durable_predecessor_identity_mismatch_is_rejected_before_offer() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let record = lock_recover(&fixture.state.record).clone();
    let current: crate::pty_custody::CustodyDurableRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.pty_custody(record.terminal_id))
        .unwrap();
    let holder = current.armed_lease().unwrap().holder;
    let mismatch = armed_custody_for_test(&record, holder);
    fixture
        .state
        .namespace
        .write_json(
            &fixture.state.paths.pty_custody(record.terminal_id),
            &mismatch,
        )
        .unwrap();
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);

    assert_adoption_closed_before_offer(&mut wire, MISMATCH_ADOPTION_DETAIL).await;
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn disconnected_predecessor_rejects_changed_holder_with_same_ids() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    disconnect_predecessor(&fixture).await;
    rewrite_predecessor(&fixture, |lease| {
        lease.holder.start_time = lease
            .holder
            .start_time
            .saturating_add(CUSTODY_SUCCESSOR_INCREMENT);
    });
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);

    assert_adoption_closed_before_offer(&mut wire, MISMATCH_ADOPTION_DETAIL).await;
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn disconnected_predecessor_rejects_changed_digest_with_same_ids() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    disconnect_predecessor(&fixture).await;
    rewrite_predecessor(&fixture, |lease| {
        lease.recovery_identity_sha256 =
            crate::sha256_digest::Sha256Digest::repeated(FORGED_PREDECESSOR_DIGEST_BYTE);
    });
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);

    assert_adoption_closed_before_offer(&mut wire, MISMATCH_ADOPTION_DETAIL).await;
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
async fn disconnect_predecessor(fixture: &Fixture) {
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(predecessor).await;
}

#[cfg(target_os = "macos")]
pub(super) fn rewrite_predecessor(
    fixture: &Fixture,
    rewrite: impl FnOnce(&mut crate::pty_custody::CustodyDurableLease),
) {
    let record = lock_recover(&fixture.state.record).clone();
    let path = fixture.state.paths.pty_custody(record.terminal_id);
    let mut durable: crate::pty_custody::CustodyDurableRecord =
        fixture.state.namespace.read_json(&path).unwrap();
    let mut lease = durable.armed_lease().unwrap();
    rewrite(&mut lease);
    durable.state = crate::pty_custody::CustodyDurableState::Armed {
        lease_id: lease.lease_id,
        offer_id: lease.offer_id,
        holder: lease.holder,
        controller_generation: lease.controller_generation,
        descriptor_set: lease.descriptor_set,
        profile: lease.profile,
        recovery_identity_sha256: lease.recovery_identity_sha256,
        guardian: lease.guardian,
    };
    fixture.state.namespace.write_json(&path, &durable).unwrap();
}

#[cfg(target_os = "macos")]
pub(super) async fn assert_adoption_closed_before_offer(wire: &mut SuccessorAdoptionWire, detail: &str) {
    let error = wire
        .peer
        .as_mut()
        .unwrap()
        .read::<crate::pty_custody::CustodyOffer>()
        .await
        .expect_err(detail);
    assert!(matches!(
        error.downcast_ref::<ProtocolError>(),
        Some(ProtocolError::EndOfStream)
    ));
}
