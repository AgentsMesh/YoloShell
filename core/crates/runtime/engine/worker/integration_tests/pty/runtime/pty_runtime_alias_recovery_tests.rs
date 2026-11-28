use crate::worker::tests::*;
const HOST_RELEASE_GENERATION_INCREMENT: u64 = 1;
const HOST_RELEASE_RETRY_GENERATION: u64 =
    FIXTURE_CONTROLLER_GENERATION + HOST_RELEASE_GENERATION_INCREMENT;
const HOST_RELEASE_PENDING_DETAIL: &str =
    "the failed predecessor Host release did not remain retryable";
const HOST_RELEASE_RETRY_TIMEOUT_DETAIL: &str =
    "the successor Host did not complete exact PTY release";
const HOST_RELEASE_CLOSE_PANIC_DETAIL: &str = "injected local PTY alias closure panic";
const HOST_RELEASE_CLOSE_RECOVERY_DETAIL: &str =
    "local PTY alias closure panic permanently fenced release state";
const HOST_RELEASE_PENDING_RETRY_DETAIL: &str =
    "pending local PTY alias closure did not preserve an open retry state";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_disconnect_during_release_retries_on_successor_custody() {
    let fixture = Fixture::new();
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let (lease, peer) = test_custody_lease(&fixture, predecessor);
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(lease.release_handle());
    let hold = tokio::spawn(lease.hold(Arc::clone(&fixture.state)));
    fixture.state.reader_lifecycle.mark_drained();
    let output = pty_output_progress::ReaderBackedPtyOutput::observed_with_release_custody(
        Arc::new(std::sync::atomic::AtomicUsize::new(usize::default())),
        Arc::clone(&fixture.state.reader_lifecycle),
        Arc::clone(&fixture.state.pty_alias_custody),
        runtime_release_authority(&lock_recover(&fixture.state.record)),
    );
    let recovery = output.recovery();
    let (first, mut recovery) = poll_alias_release(recovery).await;
    assert!(matches!(
        first.unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Pending
    ));
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    peer.read::<crate::pty_custody::CustodyControl>()
        .await
        .unwrap();
    drop(peer);
    assert!(hold.await.unwrap().is_err());
    let (failed, returned) = poll_alias_release(recovery).await;
    recovery = returned;
    assert!(
        matches!(
            failed.unwrap(),
            pty_output_progress::MasterAliasReleasePoll::Pending
        ),
        "{HOST_RELEASE_PENDING_DETAIL}"
    );

    let successor = secondary_custody_authority::ReadyWorkerCustodyIdentity::fresh_for_test(
        HOST_RELEASE_RETRY_GENERATION,
    );
    let (lease, peer) = test_custody_lease(&fixture, successor);
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(lease.release_handle());
    let hold = tokio::spawn(lease.hold(Arc::clone(&fixture.state)));
    let (retried, returned) = poll_alias_release(recovery).await;
    recovery = returned;
    assert!(matches!(
        retried.unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Pending
    ));
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
    peer.write(&crate::pty_custody::CustodyControlAck::released(&control).unwrap())
        .await
        .unwrap();

    let completed = tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + CUSTODY_RELEASE_TIMEOUT;
        loop {
            match recovery.release_master_aliases() {
                Ok(pty_output_progress::MasterAliasReleasePoll::Closed(_)) => return true,
                Ok(pty_output_progress::MasterAliasReleasePoll::Pending)
                    if Instant::now() < deadline =>
                {
                    std::thread::yield_now();
                }
                _ => return false,
            }
        }
    })
    .await
    .unwrap();
    assert!(completed, "{HOST_RELEASE_RETRY_TIMEOUT_DETAIL}");
    assert!(hold.await.unwrap().is_ok());
}

#[test]
fn local_closure_panic_restores_open_release_state() {
    let fixture = Fixture::new();
    let custody = pty_runtime_alias_custody::PtyRuntimeAliasCustody::new_for_test();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = custody
            .release(|| -> anyhow::Result<_> { panic!("{HOST_RELEASE_CLOSE_PANIC_DETAIL}") });
    }));
    assert!(panic.is_err());

    let authority = runtime_release_authority(&lock_recover(&fixture.state.record));
    let recovered = custody
        .release(|| {
            authority
                .take_local_aliases_closed()
                .map(pty_runtime_alias_custody::PtyRuntimeLocalAliasRelease::closed)
        })
        .unwrap();
    assert!(
        matches!(
            recovered,
            pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
        ),
        "{HOST_RELEASE_CLOSE_RECOVERY_DETAIL}"
    );
}

#[test]
fn pending_local_closure_preserves_open_release_state() {
    let fixture = Fixture::new();
    let custody = pty_runtime_alias_custody::PtyRuntimeAliasCustody::new_for_test();
    let first = custody
        .release(|| Ok(pty_runtime_alias_custody::PtyRuntimeLocalAliasRelease::Pending))
        .unwrap();
    assert!(matches!(
        first,
        pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
    ));

    let authority = runtime_release_authority(&lock_recover(&fixture.state.record));
    let invoked = std::sync::atomic::AtomicBool::new(false);
    let retried = custody
        .release(|| {
            invoked.store(true, Ordering::Release);
            authority
                .take_local_aliases_closed()
                .map(pty_runtime_alias_custody::PtyRuntimeLocalAliasRelease::closed)
        })
        .unwrap();
    assert!(matches!(
        retried,
        pty_runtime_alias_custody::PtyRuntimeAliasReleasePoll::Pending
    ));
    assert!(
        invoked.load(Ordering::Acquire),
        "{HOST_RELEASE_PENDING_RETRY_DETAIL}"
    );
}

pub(super) async fn poll_alias_release(
    mut recovery: pty_output_progress::PtyOutputRecovery,
) -> (
    anyhow::Result<pty_output_progress::MasterAliasReleasePoll>,
    pty_output_progress::PtyOutputRecovery,
) {
    tokio::task::spawn_blocking(move || {
        let result = recovery.release_master_aliases();
        (result, recovery)
    })
    .await
    .unwrap()
}
