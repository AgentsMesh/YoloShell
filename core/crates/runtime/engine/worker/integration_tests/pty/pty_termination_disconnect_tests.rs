use crate::worker::tests::*;
const DISCONNECTED_TERMINATION_COMMIT_DETAIL: &str =
    "disconnected owner termination did not commit PTY alias-release authority";
const DISCONNECTED_TERMINATION_TYPE_DETAIL: &str =
    "disconnected owner termination lost its authoritative settlement type";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnected_owner_commits_alias_release_before_waiting_for_settlement() {
    let fixture = Fixture::new();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let publication = Arc::clone(&settlement);
    let (commands, receiver) = mpsc::channel();
    drop(receiver);
    fixture
        .state
        .install_pty_owner_for_test(pty_owner_handle::PtyOwnerHandle::for_test(
            commands, settlement,
        ));
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .arm();

    let state = Arc::clone(&fixture.state);
    let termination = tokio::spawn(async move {
        let permit = state.admit_system_mutation().await;
        terminate_pty_processes(&state, permit, true).await
    });
    tokio::time::timeout(
        CONTROL_BARRIER_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .terminate_before_delivery
            .wait_until_reached(),
    )
    .await
    .expect(DISCONNECTED_TERMINATION_COMMIT_DETAIL);

    assert!(
        fixture
            .state
            .pty_alias_custody
            .alias_closure_is_committed_for_test(),
        "{DISCONNECTED_TERMINATION_COMMIT_DETAIL}"
    );
    assert_task_pending(&termination).await;
    publication
        .set(Err(
            pty_owner_handle::PtyOwnerCommandFailure::CleanupDeferred(
                termination_timing::CapabilitySettlementDeferred,
            ),
        ))
        .unwrap();
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .resume();

    let error = termination.await.unwrap().unwrap_err();
    assert!(
        error
            .downcast_ref::<termination_timing::CapabilitySettlementDeferred>()
            .is_some(),
        "{DISCONNECTED_TERMINATION_TYPE_DETAIL}"
    );
}
