use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const RELEASED_RETRY_CLOSE_DETAIL: &str =
    "released successor unexpectedly requested another local-close operation";
#[cfg(target_os = "macos")]
const RELEASED_RETRY_TIMEOUT_DETAIL: &str =
    "released successor did not acknowledge the exact local closure";
#[cfg(target_os = "macos")]
const RELEASED_CONTROLLER_AUTHORITY_DETAIL: &str =
    "released successor unexpectedly acquired terminal controller authority";
#[cfg(target_os = "macos")]
const RELEASED_TERMINATION_AUTHORITY_DETAIL: &str =
    "released successor did not retain terminal shutdown authority";
#[cfg(target_os = "macos")]
const RELEASED_SHUTDOWN_TIMEOUT_DETAIL: &str =
    "released successor left the terminal Worker running";
#[cfg(target_os = "macos")]
const RELEASED_SUCCESS_EXIT_CODE: i32 = 0;

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn released_successor_adopts_without_reacquiring_master_custody() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install_released(&fixture).await;
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    assert_eq!(
        offer.descriptor_set,
        crate::pty_custody::CustodyDescriptorSet::ReleasedSuccessor
    );
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    let controller = AuthenticatedController::new(wire.request.controller_generation);
    let Err(rejected) = fixture.state.admit_controller_mutation(controller).await else {
        panic!("{RELEASED_CONTROLLER_AUTHORITY_DETAIL}");
    };
    assert_eq!(
        rejected.message,
        secondary_custody_authority::CUSTODY_UNAVAILABLE_DETAIL,
        "{RELEASED_CONTROLLER_AUTHORITY_DETAIL}"
    );

    let custody = Arc::clone(&fixture.state.pty_alias_custody);
    let first = custody
        .release(|| -> anyhow::Result<_> { panic!("{RELEASED_RETRY_CLOSE_DETAIL}") })
        .unwrap();
    assert!(matches!(
        first,
        pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
    ));
    let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
    peer.write(&crate::pty_custody::CustodyControlAck::released(&control).unwrap())
        .await
        .unwrap();

    let deadline = tokio::time::Instant::now() + ADOPTION_TEST_TIMEOUT;
    loop {
        match custody.release(|| -> anyhow::Result<_> { panic!("{RELEASED_RETRY_CLOSE_DETAIL}") }) {
            Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Closed(_)) => break,
            Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending)
                if tokio::time::Instant::now() < deadline =>
            {
                tokio::task::yield_now().await;
            }
            _ => panic!("{RELEASED_RETRY_TIMEOUT_DETAIL}"),
        }
    }
    adoption.await.unwrap().unwrap();
    assert!(lock_recover(&fixture.state.secondary_custody).is_none());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn released_successor_retains_committed_terminal_shutdown_authority() {
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
    let mut wire = SuccessorAdoptionWire::install_released(&fixture).await;
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    assert_eq!(
        fixture.state.termination_authority(),
        Some(TerminationAuthority::ReleasedRecovery),
        "{RELEASED_TERMINATION_AUTHORITY_DETAIL}"
    );

    let custody = Arc::clone(&fixture.state.pty_alias_custody);
    assert!(matches!(
        custody
            .release(|| -> anyhow::Result<_> { panic!("{RELEASED_RETRY_CLOSE_DETAIL}") })
            .unwrap(),
        pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
    ));
    let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
    peer.write(&crate::pty_custody::CustodyControlAck::released(&control).unwrap())
        .await
        .unwrap();
    wait_for_released_closure(&custody).await;
    adoption.await.unwrap().unwrap();

    fixture.state.reader_lifecycle.mark_drained();
    let shutdown = fixture.state.shutdown.notified();
    let state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || {
        finalize_child_exit(
            &state,
            Some(RELEASED_SUCCESS_EXIT_CODE),
            None,
            Duration::ZERO,
        );
    })
    .await
    .unwrap();
    tokio::time::timeout(ADOPTION_TEST_TIMEOUT, shutdown)
        .await
        .expect(RELEASED_SHUTDOWN_TIMEOUT_DETAIL);
    assert!(fixture.state.response_shutdown_is_committed());
    assert_eq!(
        lock_recover(&fixture.state.exit)
            .as_ref()
            .map(|exit| exit.reason),
        Some(ExitReason::UserTerminate)
    );
}

#[cfg(target_os = "macos")]
async fn wait_for_released_closure(
    custody: &Arc<pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
) {
    let deadline = tokio::time::Instant::now() + ADOPTION_TEST_TIMEOUT;
    loop {
        match custody.release(|| -> anyhow::Result<_> { panic!("{RELEASED_RETRY_CLOSE_DETAIL}") }) {
            Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Closed(_)) => return,
            Ok(pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending)
                if tokio::time::Instant::now() < deadline =>
            {
                tokio::task::yield_now().await;
            }
            _ => panic!("{RELEASED_RETRY_TIMEOUT_DETAIL}"),
        }
    }
}
