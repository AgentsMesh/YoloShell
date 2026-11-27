use super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHolderIdentity, CustodyLeaseId, CustodyRuntimeIdentity,
};
use uuid::Uuid;

#[path = "../../test_support/unit/preparation_test_support.rs"]
mod support;
use support::distinct_file_identities;

const AUTH_TOKEN: &str = "guardian-preparation-token";
const BOOT_ID: &str = "guardian-preparation-boot";
const CONTROLLER_GENERATION: u64 = 7;
const WORKER_PID: u32 = 41_001;
const WORKER_START_TIME: u64 = 51_001;
const CHILD_PID: u32 = 41_002;
const CHILD_START_TIME: u64 = 51_002;
const HOST_PID: u32 = 41_003;
const HOST_START_TIME: u64 = 51_003;
const GUARDIAN_PID: u32 = 41_004;
const GUARDIAN_START_TIME: u64 = 51_004;
const READER_HANDLE: u64 = 61_001;
const WRITER_HANDLE: u64 = 61_002;
const DIGEST_BYTE: u8 = 0x6a;

struct Fixture {
    pending: CustodyDurableRecord,
    prepared: CustodyDurableRecord,
    request: CustodyRequest,
    offer: CustodyOffer,
    binding: CustodyGuardianBinding,
    identity: FileIdentity,
    replacement_identity: FileIdentity,
}

impl Fixture {
    fn new() -> Self {
        let runtime = runtime();
        let pending = CustodyDurableRecord::pending(runtime.clone());
        let request = CustodyRequest::new(
            CustodyLeaseId::new(),
            runtime,
            holder(HOST_PID, HOST_START_TIME),
            AUTH_TOKEN.to_owned(),
            CONTROLLER_GENERATION,
        );
        let offer = CustodyOffer::new_with_profile(
            &request,
            CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
            crate::pty_custody::CustodyProfile::ActiveRunning,
            crate::sha256_digest::Sha256Digest::repeated(DIGEST_BYTE),
        )
        .unwrap();
        let binding = binding(&request, &offer);
        let prepared = pending.prepare_guardian(&request, &offer, binding).unwrap();
        let (identity, replacement_identity) = distinct_file_identities();
        Self {
            pending,
            prepared,
            request,
            offer,
            binding,
            identity,
            replacement_identity,
        }
    }

    fn preparation(&self) -> CustodyGuardianPreparation {
        CustodyGuardianPreparation::new(self.binding, self.identity, self.identity, self.identity)
    }
}

#[test]
fn exact_prepared_record_and_identity_retry_is_idempotent() {
    let fixture = Fixture::new();
    assert_eq!(
        exact_retry(
            &fixture.prepared,
            fixture.identity,
            &fixture.request,
            &fixture.offer,
            fixture.binding,
            fixture.preparation(),
        )
        .unwrap(),
        fixture.preparation()
    );
}

#[test]
fn stale_record_cannot_reuse_a_retained_preparation() {
    let fixture = Fixture::new();
    assert!(
        exact_retry(
            &fixture.pending,
            fixture.identity,
            &fixture.request,
            &fixture.offer,
            fixture.binding,
            fixture.preparation(),
        )
        .is_err()
    );
}

#[test]
fn same_value_replacement_identity_invalidates_the_retry() {
    let fixture = Fixture::new();
    assert!(
        exact_retry(
            &fixture.prepared,
            fixture.replacement_identity,
            &fixture.request,
            &fixture.offer,
            fixture.binding,
            fixture.preparation(),
        )
        .is_err()
    );
}

#[test]
fn different_retained_binding_cannot_reuse_the_record() {
    let fixture = Fixture::new();
    let changed = binding(&fixture.request, &fixture.offer);
    let retained = CustodyGuardianPreparation::new(
        changed,
        fixture.identity,
        fixture.identity,
        fixture.identity,
    );
    assert_ne!(changed, fixture.binding);
    assert!(
        exact_retry(
            &fixture.prepared,
            fixture.identity,
            &fixture.request,
            &fixture.offer,
            fixture.binding,
            retained,
        )
        .is_err()
    );
}

#[test]
fn different_prepared_candidate_cannot_reuse_a_matching_ticket() {
    let fixture = Fixture::new();
    let changed = binding(&fixture.request, &fixture.offer);
    let retained = CustodyGuardianPreparation::new(
        changed,
        fixture.identity,
        fixture.identity,
        fixture.identity,
    );
    assert!(
        exact_retry(
            &fixture.prepared,
            fixture.identity,
            &fixture.request,
            &fixture.offer,
            changed,
            retained,
        )
        .is_err()
    );
}

fn runtime() -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START_TIME,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START_TIME,
        boot_id: BOOT_ID.to_owned(),
    }
}

fn binding(request: &CustodyRequest, offer: &CustodyOffer) -> CustodyGuardianBinding {
    CustodyGuardianBinding::new(
        Uuid::new_v4(),
        holder(GUARDIAN_PID, GUARDIAN_START_TIME),
        request,
        offer,
    )
    .unwrap()
}

fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}
