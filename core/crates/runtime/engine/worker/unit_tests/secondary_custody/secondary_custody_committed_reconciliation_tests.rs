use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn lost_ready_acknowledgement_reconciles_exact_committed_successor_without_authorizing() {
    assert_committed_adoption_reconciles(CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn acknowledged_post_ready_rollback_preserves_reserved_successor_generation() {
    assert_committed_adoption_reconciles(CommittedAdoptionFailure::ReleaseAcknowledged).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn lost_rollback_acknowledgement_reconciles_without_retiring_live_custody() {
    assert_committed_adoption_reconciles(CommittedAdoptionFailure::ReleaseAcknowledgementLost)
        .await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_post_ready_adoption_retains_exact_reconciliation_evidence() {
    assert_committed_adoption_reconciles(CommittedAdoptionFailure::CancelledBeforePromotion).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn cancelled_post_promotion_adoption_does_not_regress_persisted_generation() {
    assert_committed_adoption_reconciles(CommittedAdoptionFailure::CancelledAfterPromotion).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn busy_lifecycle_reconciliation_is_retryable_without_losing_committed_evidence() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let lifecycle = fixture
        .state
        .paths
        .try_acquire_session_lifecycle_lock(fixture.terminal_id())
        .unwrap()
        .unwrap();
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert!(committed_custody_artifact(&fixture) == artifact);
    drop(lifecycle);
    reconcile_committed_adoption(&fixture).await.unwrap();
    let identity = fixture
        .state
        .secondary_custody_authority
        .predecessor_identity()
        .unwrap();
    assert!(identity.matches_durable(artifact.0.armed_lease().unwrap()));
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn next_adoption_reconciles_committed_predecessor_before_holder_liveness_rejection() {
    let fixture = Fixture::new();
    let mut wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let (worker, peer) = UnixStream::pair().unwrap();
    wire.worker = Some(worker);
    wire.peer = Some(crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap());
    wire.request.controller_generation = wire
        .request
        .controller_generation
        .saturating_add(CUSTODY_SUCCESSOR_INCREMENT);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    assert_adoption_closed_before_offer(&mut wire, FENCED_ADOPTION_DETAIL).await;
    await_adoption_failure(adoption).await;
    let identity = fixture
        .state
        .secondary_custody_authority
        .predecessor_identity()
        .unwrap();
    assert!(identity.matches_durable(artifact.0.armed_lease().unwrap()));
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(identity)
    );
    assert!(committed_custody_artifact(&fixture) == artifact);
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn silent_post_ready_rollback_times_out_without_losing_reconciliation_evidence() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .fail_next_controller_persist();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition =
        wire.peer.as_mut().unwrap().read().await.unwrap();
    assert!(matches!(disposition,
        crate::pty_custody::CustodyWorkerReadyDisposition::Release { control } if control.matches_release(&offer)));
    tokio::time::pause();
    tokio::time::advance(crate::session_timing::WORKER_READY_TIMEOUT).await;
    await_adoption_failure(adoption).await;
    reconcile_committed_adoption(&fixture).await.unwrap();
    assert!(lock_recover(&fixture.state.secondary_custody).is_some());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn committed_runtime_promotion_without_memory_publication_is_reconciled_exactly() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let original = fixture.state.controller_generation.load(Ordering::Acquire);
    let published = crate::custody_promotion::publish_exact(
        fixture.state.namespace.lease(),
        &fixture.state.paths,
        &artifact.0,
        artifact.1,
    )
    .unwrap();
    assert!(published.controller_generation > original);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        original
    );
    reconcile_committed_adoption(&fixture).await.unwrap();
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        published.controller_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        published.controller_generation
    );
    assert!(*lock_recover(&fixture.state.record) == published);
    assert!(
        !fixture
            .state
            .custody_authorizes(ControllerGeneration(published.controller_generation))
    );
    assert!(committed_custody_artifact(&fixture) == artifact);
}
