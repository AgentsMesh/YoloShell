use crate::worker::tests::*;
const RUNTIME_RELEASE_ACK_TIMEOUT_DETAIL: &str =
    "runtime release did not observe the exact Host acknowledgement";
const RUNTIME_RELEASE_FINAL_PROOF_DETAIL: &str =
    "exact Host release did not produce the final closure proof";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_alias_closure_remains_pending_until_exact_host_release_ack() {
    let fixture = Fixture::new();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let (lease, peer) = test_custody_lease(&fixture, identity);
    let expected_offer = lease.offer_for_test();
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(lease.release_handle());
    let hold_state = Arc::clone(&fixture.state);
    let hold = tokio::spawn(lease.hold(hold_state));
    fixture.state.reader_lifecycle.mark_drained();
    let authority = runtime_release_authority(&lock_recover(&fixture.state.record));
    let output = pty_output_progress::ReaderBackedPtyOutput::observed_with_release_custody(
        Arc::new(std::sync::atomic::AtomicUsize::new(usize::default())),
        Arc::clone(&fixture.state.reader_lifecycle),
        Arc::clone(&fixture.state.pty_alias_custody),
        authority,
    );
    let mut recovery = output.recovery();

    let (first, returned) = tokio::task::spawn_blocking(move || {
        let first = recovery.release_master_aliases();
        (first, recovery)
    })
    .await
    .unwrap();
    recovery = returned;
    assert!(matches!(
        first.unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Pending
    ));
    assert!(lock_recover(&fixture.state.master).is_none());
    assert!(lock_recover(&fixture.state.writer).is_none());

    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
    assert!(control.matches_release(&expected_offer));
    let acknowledgement = crate::pty_custody::CustodyControlAck::released(&control).unwrap();
    peer.write(&acknowledgement).await.unwrap();

    let (second, mut recovery) = tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + CUSTODY_RELEASE_TIMEOUT;
        loop {
            match recovery.release_master_aliases() {
                Ok(pty_output_progress::MasterAliasReleasePoll::Pending)
                    if Instant::now() < deadline =>
                {
                    std::thread::yield_now();
                }
                Ok(pty_output_progress::MasterAliasReleasePoll::Pending) => {
                    return (
                        Err(anyhow::anyhow!(RUNTIME_RELEASE_ACK_TIMEOUT_DETAIL)),
                        recovery,
                    );
                }
                result => return (result, recovery),
            }
        }
    })
    .await
    .unwrap();
    match second.unwrap() {
        pty_output_progress::MasterAliasReleasePoll::Closed(_) => {}
        _ => panic!("{RUNTIME_RELEASE_FINAL_PROOF_DETAIL}"),
    }
    assert!(
        fixture
            .state
            .pty_alias_custody
            .released_successor_adoptable()
    );
    assert!(matches!(
        recovery.release_master_aliases().unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Delivered
    ));
    drop(peer);
    tokio::time::timeout(CUSTODY_RELEASE_TIMEOUT, hold)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
