use crate::worker::tests::*;
const ALIAS_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
const ALIAS_RELEASE_PENDING_DETAIL: &str =
    "PTY alias closure bypassed a mutation permit that still held the gate";
const ALIAS_RELEASE_DRIVER_DETAIL: &str =
    "durable PTY alias closure did not progress through FIFO admission";

async fn wait_for_alias_driver(fixture: &Fixture) {
    tokio::time::timeout(ALIAS_RELEASE_TIMEOUT, async {
        while lock_recover(&fixture.state.master).is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(ALIAS_RELEASE_DRIVER_DETAIL);
    assert!(lock_recover(&fixture.state.writer).is_none());
}

async fn wait_for_alias_waiter(fixture: &Fixture) {
    tokio::time::timeout(
        ALIAS_RELEASE_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .alias_closure_waiter_queued
            .wait_until_reached(),
    )
    .await
    .expect(ALIAS_RELEASE_DRIVER_DETAIL);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_release_queues_durable_closure_behind_its_permit() {
    let fixture = Fixture::new();
    let permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    let prepared = fixture.state.prepare_controller_alias_release(&permit);
    fixture
        .state
        .commit_controller_alias_release(&permit, prepared);

    assert_eq!(
        fixture
            .state
            .pty_alias_custody
            .close_local_aliases()
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    wait_for_alias_waiter(&fixture).await;
    assert!(lock_recover(&fixture.state.master).is_some());
    assert!(lock_recover(&fixture.state.writer).is_some());

    drop(permit);
    wait_for_alias_driver(&fixture).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn busy_unrelated_mutation_defers_alias_closure_without_side_effects() {
    let fixture = Fixture::new();
    let guard = Arc::clone(&fixture.state.mutation_gate).lock_owned().await;
    assert_eq!(
        fixture
            .state
            .pty_alias_custody
            .close_local_aliases()
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    assert!(lock_recover(&fixture.state.master).is_some());
    assert!(lock_recover(&fixture.state.writer).is_some());

    drop(guard);
    assert_eq!(
        fixture
            .state
            .pty_alias_custody
            .close_local_aliases()
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn system_authority_committed_during_poll_race_uses_fifo_driver() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .arm();
    let custody = Arc::clone(&fixture.state.pty_alias_custody);
    let (finished, completion) = mpsc::channel();
    let release = std::thread::spawn(move || {
        let _ = finished.send(custody.close_local_aliases());
    });
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .wait_until_reached();
    let permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .begin_system_termination(&permit, ExitReason::Lost);
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .resume();

    let result = completion
        .recv_timeout(ALIAS_RELEASE_TIMEOUT)
        .expect(ALIAS_RELEASE_PENDING_DETAIL)
        .unwrap();
    assert_eq!(
        result,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    release.join().unwrap();
    wait_for_alias_waiter(&fixture).await;
    assert!(lock_recover(&fixture.state.master).is_some());
    drop(permit);
    wait_for_alias_driver(&fixture).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_authority_committed_during_poll_race_uses_fifo_driver() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .arm();
    let custody = Arc::clone(&fixture.state.pty_alias_custody);
    let (finished, completion) = mpsc::channel();
    let release = std::thread::spawn(move || {
        let _ = finished.send(custody.close_local_aliases());
    });
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .wait_until_reached();
    let permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    let prepared = fixture.state.prepare_controller_alias_release(&permit);
    fixture
        .state
        .commit_controller_alias_release(&permit, prepared);
    fixture
        .state
        .control_test_hooks
        .alias_closure_before_try_admission
        .resume();

    assert_eq!(
        completion
            .recv_timeout(ALIAS_RELEASE_TIMEOUT)
            .expect(ALIAS_RELEASE_PENDING_DETAIL)
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Pending
    );
    release.join().unwrap();
    wait_for_alias_waiter(&fixture).await;
    assert!(lock_recover(&fixture.state.master).is_some());
    drop(permit);
    wait_for_alias_driver(&fixture).await;
}
