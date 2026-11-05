use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const ACTIVATION_LEASE_DIGEST_BYTE: u8 = 0x71;
#[cfg(target_os = "macos")]
const ACTIVATION_READER_HANDLE: u64 = 70_001;
#[cfg(target_os = "macos")]
const ACTIVATION_WRITER_HANDLE: u64 = 70_002;

#[cfg(target_os = "macos")]
pub(super) fn pending_activation_lease(
    fixture: &Fixture,
) -> (
    WorkerCustodyLease,
    crate::pty_custody::CustodyUnixStream,
    crate::pty_custody::CustodyOffer,
) {
    let record = lock_recover(&fixture.state.record).clone();
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        holder,
        record.auth_token,
        record.controller_generation,
    );
    let offer = crate::pty_custody::test_support::custody_offer(
        &request,
        crate::pty_custody::CustodyDescriptorSet::macos_process_capability(
            ACTIVATION_READER_HANDLE,
            ACTIVATION_WRITER_HANDLE,
        ),
        crate::sha256_digest::Sha256Digest::repeated(ACTIVATION_LEASE_DIGEST_BYTE),
    );
    let (worker, peer) = UnixStream::pair().unwrap();
    let worker = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    let peer = crate::pty_custody::CustodyUnixStream::from_local(peer).unwrap();
    let lease = WorkerCustodyLease::new(
        worker,
        offer.clone(),
        holder,
        record.controller_generation,
        None,
    )
    .unwrap();
    (lease, peer, offer)
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn worker_custody_activation_is_consumed_exactly_once() {
    let fixture = Fixture::new();
    let generation = lock_recover(&fixture.state.record).controller_generation;
    let (lease, mut peer, offer) = pending_activation_lease(&fixture);

    let lease = lease.activate().await.unwrap();
    receive_exact_worker_activation(&mut peer, &offer, generation).await;
    assert!(lease.activate().await.is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn worker_custody_hold_rejects_a_nonactivated_lease() {
    let fixture = Fixture::new();
    let (lease, _peer, _offer) = pending_activation_lease(&fixture);

    assert!(lease.hold(Arc::clone(&fixture.state)).await.is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn initial_post_ready_guard_drop_fences_installed_authority() {
    let fixture = Fixture::new();
    fixture.state.fence_all_custody().await;
    let (lease, _peer, _offer) = pending_activation_lease(&fixture);
    let identity = lease.identity();
    let fence =
        secondary_custody_adoption_fence::CustodyAdoptionFence::armed(Arc::clone(&fixture.state));
    let permit = fixture.state.admit_system_mutation().await;
    fixture
        .state
        .install_initial_ready_custody(&permit, &lease)
        .unwrap();
    assert_eq!(
        fixture.state.secondary_custody_authority.active_for_test(),
        Some(identity)
    );
    assert_eq!(
        fixture.state.pty_alias_custody.release_identity_for_test(),
        Some(identity)
    );

    drop(fence);
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .active_for_test()
            .is_none()
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .release_identity_for_test()
            .is_none()
    );
    drop(permit);
}
