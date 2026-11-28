use crate::worker::tests::*;
const PRE_ACTIVATION_TEST_GENERATION: u64 = 1;
const PRE_ACTIVATION_TEST_DIGEST_BYTE: u8 = 0x6c;
const PRE_ACTIVATION_TEST_AUTH_TOKEN: &str = "pre-activation-custody";
const PRE_ACTIVATION_READER_HANDLE: u64 = 70_005;
const PRE_ACTIVATION_TEST_RELEASE_DETAIL: &str =
    "pre-activation rollback emitted an activation decision";
const PRE_ACTIVATION_WRITER_HANDLE: u64 = 70_006;

#[tokio::test]
async fn pre_activation_rollback_accepts_only_the_exact_released_ack() {
    let (lease, peer, offer) = pre_activation_lease();
    let rollback = tokio::spawn(lease.rollback_before_activation());
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let control = read_pre_activation_release(&mut peer).await;
    assert!(control.matches_release(&offer));
    peer.write(&crate::pty_custody::CustodyControlAck::released(&control).unwrap())
        .await
        .unwrap();

    rollback.await.unwrap().unwrap();

    let (lease, peer, _) = pre_activation_lease();
    let rollback = tokio::spawn(lease.rollback_before_activation());
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let control = read_pre_activation_release(&mut peer).await;
    peer.write(&stale_offer_ack(&control).unwrap())
        .await
        .unwrap();
    assert!(rollback.await.unwrap().is_err());
}

#[tokio::test]
async fn pre_activation_rollback_eof_and_cancellation_fail_closed() {
    let (lease, peer, _) = pre_activation_lease();
    let eof = tokio::spawn(lease.rollback_before_activation());
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    read_pre_activation_release(&mut peer).await;
    drop(peer);
    assert!(eof.await.unwrap().is_err());

    let (lease, peer, _) = pre_activation_lease();
    let cancelled = tokio::spawn(lease.rollback_before_activation());
    let mut peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    read_pre_activation_release(&mut peer).await;
    cancelled.abort();
    assert!(matches!(cancelled.await, Err(error) if error.is_cancelled()));
}

async fn read_pre_activation_release(
    peer: &mut crate::pty_custody::CustodyUnixStream,
) -> crate::pty_custody::CustodyControl {
    let disposition: crate::pty_custody::CustodyWorkerReadyDisposition = peer.read().await.unwrap();
    let crate::pty_custody::CustodyWorkerReadyDisposition::Release { control } = disposition else {
        panic!("{PRE_ACTIVATION_TEST_RELEASE_DETAIL}");
    };
    control
}

fn pre_activation_lease() -> (
    WorkerCustodyLease,
    UnixStream,
    crate::pty_custody::CustodyOffer,
) {
    let fixture = Fixture::new();
    let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(&lock_recover(
        &fixture.state.record,
    ));
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        runtime,
        holder,
        PRE_ACTIVATION_TEST_AUTH_TOKEN.to_owned(),
        PRE_ACTIVATION_TEST_GENERATION,
    );
    let offer = crate::pty_custody::test_support::custody_offer(
        &request,
        crate::pty_custody::CustodyDescriptorSet::macos_process_capability(
            PRE_ACTIVATION_READER_HANDLE,
            PRE_ACTIVATION_WRITER_HANDLE,
        ),
        crate::sha256_digest::Sha256Digest::repeated(PRE_ACTIVATION_TEST_DIGEST_BYTE),
    );
    let (worker, peer) = UnixStream::pair().unwrap();
    let stream = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    let lease = WorkerCustodyLease::new(
        stream,
        offer.clone(),
        holder,
        PRE_ACTIVATION_TEST_GENERATION,
        None,
    )
    .unwrap();
    (lease, peer, offer)
}
