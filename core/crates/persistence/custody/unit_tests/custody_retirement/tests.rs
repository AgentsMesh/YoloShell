use super::receipt_is_exact;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRetirementEvidence,
    CustodyRuntimeIdentity,
};
use uuid::Uuid;

const TEST_AUTH_TOKEN: &str = "custody-retirement-test-token";
const TEST_BOOT_ID: &str = "custody-retirement-test-boot";
const TEST_CONTROLLER_GENERATION: u64 = 7;
const CHANGED_CONTROLLER_GENERATION: u64 = 8;
const TEST_WORKER_PID: u32 = 31_001;
const TEST_WORKER_START_TIME: u64 = 41_001;
const TEST_CHILD_PID: u32 = 31_002;
const TEST_CHILD_START_TIME: u64 = 41_002;
const TEST_HOST_PID: u32 = 31_003;
const TEST_HOST_START_TIME: u64 = 41_003;
const TEST_GUARDIAN_PID: u32 = 31_004;
const TEST_GUARDIAN_START_TIME: u64 = 41_004;
const TEST_READER_HANDLE: u64 = 51_001;
const TEST_WRITER_HANDLE: u64 = 51_002;
const TEST_DIGEST_BYTE: u8 = 0x5c;
const TEST_LAST_OUTPUT_SEQUENCE: u64 = 13;
const IDENTITY_MAX_BYTES: u64 = u64::MAX;

#[test]
fn idempotent_receipt_requires_exact_runtime_lease_and_evidence() {
    let (runtime, lease, evidence, retired) = fixture();
    assert!(receipt_is_exact(&retired, &runtime, lease, evidence));

    let (changed_runtime, changed_lease, changed_evidence, changed_runtime_record) = fixture();
    assert_ne!(changed_runtime, runtime);
    assert!(changed_runtime_record.is_valid());
    assert!(!receipt_is_exact(
        &changed_runtime_record,
        &runtime,
        changed_lease,
        changed_evidence,
    ));

    let mut changed_lease = lease;
    changed_lease.lease_id = CustodyLeaseId::new();
    assert!(!receipt_is_exact(
        &retired,
        &runtime,
        changed_lease,
        evidence,
    ));

    let changed_evidence = CustodyRetirementEvidence::new(
        CHANGED_CONTROLLER_GENERATION,
        evidence.runtime_file_identity(),
        file_identity(),
        file_identity(),
        TEST_LAST_OUTPUT_SEQUENCE,
    );
    assert!(!receipt_is_exact(
        &retired,
        &runtime,
        lease,
        changed_evidence,
    ));
}

fn fixture() -> (
    CustodyRuntimeIdentity,
    crate::pty_custody::CustodyDurableLease,
    CustodyRetirementEvidence,
    CustodyDurableRecord,
) {
    let runtime = runtime();
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder(TEST_HOST_PID, TEST_HOST_START_TIME),
        TEST_AUTH_TOKEN.to_owned(),
        TEST_CONTROLLER_GENERATION,
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        crate::pty_custody::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE),
    )
    .unwrap();
    let binding = CustodyGuardianBinding::new(
        Uuid::new_v4(),
        holder(TEST_GUARDIAN_PID, TEST_GUARDIAN_START_TIME),
        &request,
        &offer,
    )
    .unwrap();
    let prepared = CustodyDurableRecord::pending(runtime.clone())
        .prepare_guardian(&request, &offer, binding)
        .unwrap();
    let armed = prepared
        .arm_prepared_guardian(&request, &offer, binding)
        .unwrap();
    let lease = armed.armed_lease().unwrap();
    let identity = file_identity();
    let evidence = CustodyRetirementEvidence::new(
        TEST_CONTROLLER_GENERATION,
        identity,
        identity,
        identity,
        TEST_LAST_OUTPUT_SEQUENCE,
    );
    let retired = armed.retire_with_evidence(lease, evidence).unwrap();
    (runtime, lease, evidence, retired)
}

fn runtime() -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START_TIME,
        child_pid: TEST_CHILD_PID,
        child_start_time: TEST_CHILD_START_TIME,
        boot_id: TEST_BOOT_ID.to_owned(),
    }
}

fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}

fn file_identity() -> FileIdentity {
    let file = tempfile::NamedTempFile::new().unwrap();
    FileIdentity::inspect_bounded(
        &file.path().metadata().unwrap(),
        file.path(),
        IDENTITY_MAX_BYTES,
    )
    .unwrap()
}
