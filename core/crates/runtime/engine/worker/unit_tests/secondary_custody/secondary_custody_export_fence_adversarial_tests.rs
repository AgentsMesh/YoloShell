use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn successor_export_fence_rolls_ready_candidate_back_and_rejects_next() {
    let fixture = Fixture::new();
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    let source = lock_recover(&fixture.state.secondary_custody)
        .clone()
        .unwrap();
    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ACTIVE_EXPORT
    );
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .wait_until_reached()
        .await;

    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    assert!(lock_recover(&fixture.state.master).is_some());
    assert!(lock_recover(&fixture.state.writer).is_some());
    assert!(lock_recover(&fixture.state.secondary_custody).is_some());
    assert!(source.lease_successor_export().is_err());

    let mut rejected = SuccessorAdoptionWire::connect_to_installed_source(&fixture);
    let rejected_adoption = rejected.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    assert_task_pending(&rejected_adoption).await;

    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    let release = wire.receive_pre_activation_release(&offer).await;
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        predecessor_generation
    );
    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ACTIVE_EXPORT
    );
    assert!(lock_recover(&fixture.state.master).is_some());
    assert!(lock_recover(&fixture.state.writer).is_some());
    assert!(lock_recover(&fixture.state.secondary_custody).is_some());

    wire.drop_received_descriptors();
    wire.acknowledge_exact_release(&release).await;
    await_adoption_failure(adoption).await;
    assert_eq!(
        source.successor_export_counts_for_test(),
        NO_SUCCESSOR_EXPORTS
    );
    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
    );
    assert!(lock_recover(&fixture.state.secondary_custody).is_none());
    await_adoption_failure(rejected_adoption).await;
    rejected.assert_offer_not_emitted().await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn successor_export_fence_waits_for_proven_gone_orphan() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    wire.receive_offer().await;
    let source = lock_recover(&fixture.state.secondary_custody)
        .clone()
        .unwrap();
    let holder = wire.request.holder();
    let matching = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Matching,
    );

    adoption.abort();
    let cancellation = tokio::time::timeout(ADOPTION_TEST_TIMEOUT, adoption)
        .await
        .unwrap()
        .unwrap_err();
    assert!(cancellation.is_cancelled());
    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ORPHAN_EXPORT
    );
    assert!(lock_recover(&fixture.state.master).is_some());
    assert!(lock_recover(&fixture.state.secondary_custody).is_some());
    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );

    drop(matching);
    let indeterminate = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    assert_eq!(
        source.successor_export_counts_for_test(),
        SINGLE_ORPHAN_EXPORT
    );

    wire.drop_received_descriptors();
    drop(indeterminate);
    let _gone = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    );
    assert_eq!(
        poll_export_fenced_alias_closure(&fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
    );
    assert_eq!(
        source.successor_export_counts_for_test(),
        NO_SUCCESSOR_EXPORTS
    );
}
