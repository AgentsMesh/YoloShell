use crate::worker::tests::*;
const CUSTODY_RELEASE_TEST_DEVICE: &str = "/dev/null";
const CUSTODY_RELEASE_TEST_CAPABILITY: &str = "worker-release-exchange";

enum HostReleaseResponse {
    Exact,
    StaleOffer,
    Eof,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worker_sends_exact_release_and_accepts_only_the_exact_released_ack() {
    let (exact_control, exact_completion, exact_hold) =
        run_release_exchange(HostReleaseResponse::Exact).await;
    assert!(exact_control.0.matches_release(&exact_control.1));
    assert!(exact_completion && exact_hold);

    let (_, stale_completion, stale_hold) =
        run_release_exchange(HostReleaseResponse::StaleOffer).await;
    assert!(!stale_completion && !stale_hold);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worker_release_eof_before_ack_fails_closed() {
    let (_, completion, hold) = run_release_exchange(HostReleaseResponse::Eof).await;
    assert!(!completion && !hold);
}

async fn run_release_exchange(
    response: HostReleaseResponse,
) -> (
    (
        crate::pty_custody::CustodyControl,
        crate::pty_custody::CustodyOffer,
    ),
    bool,
    bool,
) {
    let fixture = Fixture::new();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let (lease, peer) = test_custody_lease(&fixture, identity);
    let expected_offer = lease.offer_for_test();
    let release_handle = lease.release_handle();
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(release_handle.clone());
    let hold_state = Arc::clone(&fixture.state);
    let hold = tokio::spawn(lease.hold(hold_state));
    let completion = release_handle
        .begin(release_verifier(&lock_recover(&fixture.state.record)))
        .unwrap();
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let control: crate::pty_custody::CustodyControl = peer.read().await.unwrap();
    respond_to_release(&mut peer, &control, response).await;
    drop(peer);
    let completion_ok = tokio::task::spawn_blocking(move || {
        completion
            .recv_timeout(CUSTODY_RELEASE_TIMEOUT)
            .is_ok_and(|result| result.is_ok())
    })
    .await
    .unwrap();
    let hold_ok = tokio::time::timeout(CUSTODY_RELEASE_TIMEOUT, hold)
        .await
        .is_ok_and(|result| result.is_ok_and(|result| result.is_ok()));
    ((control, expected_offer), completion_ok, hold_ok)
}

async fn respond_to_release(
    peer: &mut crate::pty_custody::CustodyUnixStream,
    control: &crate::pty_custody::CustodyControl,
    response: HostReleaseResponse,
) {
    let acknowledgement = match response {
        HostReleaseResponse::Exact => crate::pty_custody::CustodyControlAck::released(control),
        HostReleaseResponse::StaleOffer => stale_offer_ack(control),
        HostReleaseResponse::Eof => None,
    };
    if let Some(acknowledgement) = acknowledgement {
        peer.write(&acknowledgement).await.unwrap();
    }
}

pub(super) fn stale_offer_ack(
    control: &crate::pty_custody::CustodyControl,
) -> Option<crate::pty_custody::CustodyControlAck> {
    let crate::pty_custody::CustodyControl::Release {
        lease_id,
        closure_identity_sha256,
        settlement,
        ..
    } = control
    else {
        return None;
    };
    Some(crate::pty_custody::CustodyControlAck::Released {
        lease_id: *lease_id,
        offer_id: crate::pty_custody::CustodyOfferId::new(),
        closure_identity_sha256: *closure_identity_sha256,
        settlement: settlement.clone(),
    })
}

fn release_verifier(record: &RuntimeRecord) -> crate::process::PtyHostReleaseVerifier {
    let mut proof = crate::process::PtySessionProof::capture(
        CUSTODY_RELEASE_TEST_DEVICE.into(),
        CUSTODY_RELEASE_TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    proof
        .bind_runtime(record.terminal_id, record.runtime_epoch)
        .unwrap();
    let authority = proof
        .bind_master_alias_release(crate::process::ProcessIdentity {
            pid: record.child_pid,
            start_time: record.child_start_time,
        })
        .unwrap();
    authority.take_local_aliases_closed().unwrap().1
}
