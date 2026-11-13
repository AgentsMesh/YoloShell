use super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use std::os::fd::OwnedFd;

const RELEASED_TEST_AUTH_TOKEN: &str = "released-validation-auth-token";
const RELEASED_TEST_BOOT_ID: &str = "released-validation-boot";
const PREDECESSOR_GENERATION: u64 = 41;
const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;
const STALE_START_TIME_INCREMENT: u64 = 1;
const RECOVERY_DIGEST_BYTE: u8 = 0x7a;
const FOREIGN_DIGEST_BYTE: u8 = 0x4c;

struct ReleasedOfferFixture {
    predecessor: CustodyDurableRecord,
    request: CustodyRequest,
    offer: CustodyOffer,
}

#[test]
fn released_successor_accepts_exact_proven_gone_predecessor() {
    let holder = stale_holder();
    let _gone = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let fixture = released_fixture(holder);

    super::super::validation::validate_offer(
        &fixture.request,
        &fixture.offer,
        &fixture.predecessor,
    )
    .unwrap();
}

#[test]
fn released_successor_rejects_current_predecessor() {
    assert_released_rejected(released_fixture(CustodyHolderIdentity::capture().unwrap()));
}

#[test]
fn released_successor_rejects_indeterminate_predecessor() {
    let holder = stale_holder();
    let _unknown = crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Unknown,
    );
    assert_released_rejected(released_fixture(holder));
}

#[test]
fn released_successor_rejects_replayed_lease_identity() {
    let holder = stale_holder();
    let _gone = install_gone(holder);
    let mut fixture = released_fixture(holder);
    fixture.request.lease_id = fixture.predecessor.armed_lease().unwrap().lease_id;
    fixture.offer = released_offer(&fixture.request, RECOVERY_DIGEST_BYTE);
    assert_released_rejected(fixture);
}

#[test]
fn released_successor_rejects_nonadvancing_generation() {
    let holder = stale_holder();
    let _gone = install_gone(holder);
    let mut fixture = released_fixture(holder);
    fixture.request.controller_generation = PREDECESSOR_GENERATION;
    assert_released_rejected(fixture);
}

#[test]
fn released_successor_rejects_changed_recovery_digest() {
    let holder = stale_holder();
    let _gone = install_gone(holder);
    let mut fixture = released_fixture(holder);
    fixture.offer.recovery_identity_sha256 = Sha256Digest::repeated(FOREIGN_DIGEST_BYTE);
    assert_released_rejected(fixture);
}

#[test]
fn released_successor_rejects_transferred_descriptors() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(super::super::super::HostConfig::new(directory.path())).unwrap();
    let record = runtime(&host, directory.path().join(TEST_SOCKET_NAME));
    let request = successor_request(CustodyRuntimeIdentity::from_record(&record));
    let offer = released_offer(&request, RECOVERY_DIGEST_BYTE);
    let descriptor: OwnedFd = tempfile::tempfile().unwrap().into();

    let error = host
        .import_secondary_custody(&record, &offer, vec![descriptor])
        .err()
        .unwrap();

    assert!(matches!(error, WorkerRpcError::Protocol(_)));
}

fn released_fixture(predecessor_holder: CustodyHolderIdentity) -> ReleasedOfferFixture {
    let runtime = test_runtime();
    let pending = CustodyDurableRecord::pending(runtime.clone());
    let predecessor_request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        predecessor_holder,
        RELEASED_TEST_AUTH_TOKEN.to_owned(),
        PREDECESSOR_GENERATION,
    );
    let predecessor_offer = crate::host::test_custody_offer(
        &predecessor_request,
        super::platform_descriptor_set(),
        Sha256Digest::repeated(RECOVERY_DIGEST_BYTE),
    );
    let predecessor =
        crate::host::test_arm_custody(&pending, &predecessor_request, &predecessor_offer)
            .unwrap();
    let request = successor_request(runtime);
    let offer = released_offer(&request, RECOVERY_DIGEST_BYTE);
    ReleasedOfferFixture {
        predecessor,
        request,
        offer,
    }
}

fn successor_request(runtime: CustodyRuntimeIdentity) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime,
        CustodyHolderIdentity::capture().unwrap(),
        RELEASED_TEST_AUTH_TOKEN.to_owned(),
        PREDECESSOR_GENERATION.saturating_add(SUCCESSOR_GENERATION_INCREMENT),
    )
}

fn released_offer(request: &CustodyRequest, digest_byte: u8) -> CustodyOffer {
    crate::host::test_custody_offer(
        request,
        CustodyDescriptorSet::ReleasedSuccessor,
        Sha256Digest::repeated(digest_byte),
    )
}

fn test_runtime() -> CustodyRuntimeIdentity {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: process.pid,
        worker_start_time: process.start_time,
        child_pid: process.pid,
        child_start_time: process.start_time,
        boot_id: RELEASED_TEST_BOOT_ID.to_owned(),
    }
}

fn stale_holder() -> CustodyHolderIdentity {
    let current = CustodyHolderIdentity::capture().unwrap();
    CustodyHolderIdentity {
        pid: current.pid,
        start_time: current
            .start_time
            .saturating_add(STALE_START_TIME_INCREMENT),
    }
}

fn install_gone(holder: CustodyHolderIdentity) -> crate::process::ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(
        holder.pid,
        holder.start_time,
        crate::process::ProcessIdentityRelation::Gone,
    )
}

fn assert_released_rejected(fixture: ReleasedOfferFixture) {
    let result = super::super::validation::validate_offer(
        &fixture.request,
        &fixture.offer,
        &fixture.predecessor,
    );
    assert!(matches!(result, Err(WorkerRpcError::IdentityMismatch(_))));
}
