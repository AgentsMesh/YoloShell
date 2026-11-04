use crate::worker::tests::*;
const CUSTODY_AUTH_REQUEST_ID: &str = "custody-auth-without-ready";
const CUSTODY_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const CUSTODY_EOF_HOOK_TIMEOUT_DETAIL: &str =
    "custody EOF did not enter its fenced authority settlement";
const CUSTODY_HOLD_TIMEOUT_DETAIL: &str = "custody lease did not settle after peer EOF";

pub(super) async fn rejected_worker_auth(fixture: &Fixture, generation: u64) -> ErrorBody {
    let (mut client, task) = raw_worker_connection(fixture).await;
    let request = RequestEnvelope::new(
        CUSTODY_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.into(),
            controller_generation: generation,
        })
        .unwrap(),
    );
    write_frame(&mut client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    task.await.unwrap().unwrap();
    response.error.unwrap()
}

#[tokio::test]
async fn worker_auth_cannot_advance_without_exact_ready_custody() {
    let fixture = Fixture::new();
    let advance = rejected_worker_auth(&fixture, CUSTODY_SUCCESSOR_GENERATION).await;

    assert_eq!(advance.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
    assert_eq!(
        lock_recover(&fixture.state.record).controller_generation,
        FIXTURE_CONTROLLER_GENERATION
    );
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(identity).await;
    let unavailable = rejected_worker_auth(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    assert_eq!(
        unavailable.message,
        secondary_custody_authority::CUSTODY_UNAVAILABLE_DETAIL
    );
}

#[tokio::test]
async fn custody_eof_fences_controller_mutation_before_settlement_returns() {
    let fixture = Fixture::new();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let (lease, peer) = test_custody_lease(&fixture, identity);
    fixture
        .state
        .control_test_hooks
        .custody_after_unavailable
        .arm();
    let hold_state = Arc::clone(&fixture.state);
    let hold = tokio::spawn(lease.hold(hold_state));
    drop(peer);
    tokio::time::timeout(
        CUSTODY_SETTLEMENT_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .custody_after_unavailable
            .wait_until_reached(),
    )
    .await
    .expect(CUSTODY_EOF_HOOK_TIMEOUT_DETAIL);

    let mutation_state = Arc::clone(&fixture.state);
    let mutation = tokio::spawn(async move {
        mutation_state
            .admit_controller_mutation(AuthenticatedController::new(FIXTURE_CONTROLLER_GENERATION))
            .await
    });
    assert_task_pending(&mutation).await;
    fixture
        .state
        .control_test_hooks
        .custody_after_unavailable
        .resume();
    let hold_result = tokio::time::timeout(CUSTODY_SETTLEMENT_TIMEOUT, hold)
        .await
        .expect(CUSTODY_HOLD_TIMEOUT_DETAIL)
        .unwrap();
    assert!(hold_result.is_err());
    assert!(matches!(
        mutation.await.unwrap(),
        Err(error) if error.code == crate::protocol::error_code::CONFLICT
    ));
}

#[tokio::test]
async fn predecessor_eof_does_not_fence_ready_successor_custody() {
    let fixture = Fixture::new();
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let (predecessor_lease, predecessor_peer) = test_custody_lease(&fixture, predecessor);
    let predecessor_state = Arc::clone(&fixture.state);
    let predecessor_hold = tokio::spawn(predecessor_lease.hold(predecessor_state));
    let successor = secondary_custody_authority::ReadyWorkerCustodyIdentity::fresh_for_test(
        CUSTODY_SUCCESSOR_GENERATION,
    );
    let (successor_lease, _successor_peer) = test_custody_lease(&fixture, successor);
    let permit = fixture.state.admit_system_mutation().await;
    let controller = fixture
        .state
        .promote_ready_custody_for_test(&permit, &successor_lease)
        .unwrap();
    drop(permit);

    drop(predecessor_peer);
    let predecessor_result = tokio::time::timeout(CUSTODY_SETTLEMENT_TIMEOUT, predecessor_hold)
        .await
        .expect(CUSTODY_HOLD_TIMEOUT_DETAIL)
        .unwrap();
    assert!(predecessor_result.is_err());
    assert!(fixture.state.validate_controller(controller).is_ok());
    assert_eq!(
        fixture.state.secondary_custody_authority.active_for_test(),
        Some(successor)
    );
    assert_eq!(
        fixture.state.pty_alias_custody.release_identity_for_test(),
        Some(successor)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn committed_termination_survives_its_custody_release() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let permit = fixture
        .state
        .admit_controller_mutation(controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(identity).await;
    assert!(fixture.state.validate_controller(controller).is_err());

    let notified = fixture.state.shutdown.notified();
    let state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || state.notify_termination_shutdown_blocking())
        .await
        .unwrap();
    tokio::time::timeout(CUSTODY_SETTLEMENT_TIMEOUT, notified)
        .await
        .unwrap();
}

include!("../terminal/terminal_custody_authority_tests.rs");
