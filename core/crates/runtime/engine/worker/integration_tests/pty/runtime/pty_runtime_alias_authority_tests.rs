use crate::worker::tests::*;
const ALIAS_TERMINATION_COMMIT_DETAIL: &str =
    "system termination did not preserve PTY alias-release authority after cancellation";
const ALIAS_TERMINATION_PROMOTION_DETAIL: &str =
    "system-terminating worker accepted a successor custody promotion";
const ALIAS_CLOSURE_FIFO_TIMEOUT: Duration = Duration::from_secs(2);
const ALIAS_CLOSURE_FIFO_DETAIL: &str =
    "durable controller alias closure did not enter or progress through the FIFO mutation gate";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_system_termination_retains_alias_release_authority() {
    let fixture = Fixture::new();
    let state = Arc::clone(&fixture.state);
    let (committed, observed) = tokio::sync::oneshot::channel();
    let termination = tokio::spawn(async move {
        let permit = state.admit_system_mutation().await;
        state.begin_system_termination(&permit, ExitReason::Lost);
        let _ = committed.send(());
        std::future::pending::<()>().await;
    });
    observed.await.unwrap();
    termination.abort();
    assert!(termination.await.unwrap_err().is_cancelled());

    let guard = Arc::clone(&fixture.state.mutation_gate).lock_owned().await;
    let result = fixture
        .state
        .pty_alias_custody
        .close_local_aliases()
        .unwrap();
    assert_eq!(
        result,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending,
        "{ALIAS_TERMINATION_COMMIT_DETAIL}"
    );
    drop(guard);
    tokio::time::timeout(ALIAS_CLOSURE_FIFO_TIMEOUT, async {
        loop {
            if fixture
                .state
                .pty_alias_custody
                .close_local_aliases()
                .unwrap()
                == pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(ALIAS_TERMINATION_COMMIT_DETAIL);
}

#[tokio::test]
async fn system_termination_fences_successor_promotion() {
    let fixture = Fixture::new();
    let successor = secondary_custody_authority::ReadyWorkerCustodyIdentity::fresh_for_test(
        CUSTODY_SUCCESSOR_GENERATION,
    );
    let (lease, _peer) = test_custody_lease(&fixture, successor);
    let permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .begin_system_termination(&permit, ExitReason::Lost);

    let failure = fixture
        .state
        .promote_ready_custody_for_test(&permit, &lease)
        .unwrap_err();
    assert_eq!(
        failure.message,
        secondary_custody_promotion::SUCCESSOR_TERMINATION_COMMITTED_DETAIL,
        "{ALIAS_TERMINATION_PROMOTION_DETAIL}"
    );
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_controller_alias_closure_progresses_through_fifo_contention() {
    let fixture = Fixture::new();
    let permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    let alias_release = fixture.state.prepare_controller_alias_release(&permit);
    fixture
        .state
        .commit_controller_alias_release(&permit, alias_release);
    drop(permit);

    let initial = Arc::clone(&fixture.state.mutation_gate).lock_owned().await;
    assert_eq!(
        fixture
            .state
            .pty_alias_custody
            .close_local_aliases()
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    tokio::time::timeout(
        ALIAS_CLOSURE_FIFO_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .alias_closure_waiter_queued
            .wait_until_reached(),
    )
    .await
    .expect(ALIAS_CLOSURE_FIFO_DETAIL);

    let contender_gate = Arc::clone(&fixture.state.mutation_gate);
    let contender_release = Arc::new(tokio::sync::Notify::new());
    let release = Arc::clone(&contender_release);
    let (acquired, observed) = tokio::sync::oneshot::channel();
    let contender = tokio::spawn(async move {
        let _guard = contender_gate.lock_owned().await;
        let _ = acquired.send(());
        release.notified().await;
    });
    drop(initial);

    tokio::time::timeout(ALIAS_CLOSURE_FIFO_TIMEOUT, async {
        while lock_recover(&fixture.state.master).is_some() {
            tokio::task::yield_now().await;
        }
        observed.await.unwrap();
    })
    .await
    .expect(ALIAS_CLOSURE_FIFO_DETAIL);
    assert!(lock_recover(&fixture.state.master).is_none());
    assert!(lock_recover(&fixture.state.writer).is_none());
    contender_release.notify_one();
    contender.await.unwrap();
}
