use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use std::path::PathBuf;
use uuid::Uuid;

const TEST_TOKEN: &str = "custody-test-token";
const TEST_BOOT: &str = "custody-test-boot";
const TEST_DIGEST_BYTE: u8 = 0x5a;
const TEST_GENERATION: u64 = 9;
const TEST_WORKER_PID: u32 = 40_001;
const TEST_CHILD_PID: u32 = 40_002;
const TEST_WORKER_START: u64 = 50_001;
const TEST_CHILD_START: u64 = 50_002;
const TEST_HOST_PID: u32 = 40_003;
const TEST_HOST_START: u64 = 50_003;
const TEST_READER_HANDLE: u64 = 60_001;
const TEST_WRITER_HANDLE: u64 = 60_002;
const TEST_STARTED_AT_MS: i64 = 1;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;
const TEST_MACOS_DESCRIPTOR_COUNT: usize = 2;
const TEST_LINUX_DESCRIPTOR_COUNT: usize = 3;
const TEST_RELEASED_SUCCESSOR_DESCRIPTOR_COUNT: usize = 0;
const TEST_SOCKET: &str = "/tmp/yolo-custody-test.sock";
const TEST_SHELL: &str = "/bin/sh";
const TEST_DIRECTORY: &str = "/tmp";

#[test]
fn request_requires_complete_exact_runtime_identity() {
    let record = record();
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(&record),
        holder(),
        TEST_TOKEN.to_owned(),
        TEST_GENERATION,
    );
    assert!(request.is_well_formed());
    assert!(request.runtime.matches_record(&record));

    let mut stale = record;
    stale.child_start_time += 1;
    assert!(!request.runtime.matches_record(&stale));
}

#[test]
fn offer_is_bound_to_one_lease_and_runtime() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    assert!(offer.matches_request(&request));
    assert_eq!(
        offer.descriptor_set.descriptor_count(),
        TEST_MACOS_DESCRIPTOR_COUNT
    );

    let mut replay = request;
    replay.lease_id = CustodyLeaseId::new();
    assert!(!offer.matches_request(&replay));
}

#[test]
fn phase_messages_match_only_the_exact_offer_and_generation() {
    let request = request();
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let prepared = CustodyHostAck::prepared(&offer);
    let ready = CustodyHostAck::ready(&offer, TEST_GENERATION);
    let commit = CustodyWorkerCommit::new(&offer);

    assert!(prepared.matches_prepared(&offer));
    assert!(ready.matches_ready(&offer, TEST_GENERATION));
    assert!(commit.matches_offer(&offer));
    assert!(!ready.matches_ready(&offer, TEST_GENERATION + 1));

    let replay = CustodyOffer::new(&request, descriptor_set(), digest());
    assert!(!prepared.matches_prepared(&replay));
    assert!(!commit.matches_offer(&replay));
}

#[test]
fn descriptor_sets_have_exact_role_counts() {
    assert_eq!(
        descriptor_set().descriptor_count(),
        TEST_MACOS_DESCRIPTOR_COUNT
    );
    assert_eq!(
        CustodyDescriptorSet::LinuxContainment.descriptor_count(),
        TEST_LINUX_DESCRIPTOR_COUNT
    );
    assert_eq!(
        CustodyDescriptorSet::ReleasedSuccessor.descriptor_count(),
        TEST_RELEASED_SUCCESSOR_DESCRIPTOR_COUNT
    );
}

#[test]
fn protocol_rejects_unknown_fields() {
    let request = request();
    let mut value = serde_json::to_value(&request).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_owned(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<CustodyRequest>(value).is_err());
}

#[test]
fn durable_record_arms_only_from_exact_pending_runtime() {
    let runtime = CustodyRuntimeIdentity::from_record(&record());
    let pending = CustodyDurableRecord::pending(runtime.clone());
    assert!(pending.is_valid());
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder(),
        TEST_TOKEN.to_owned(),
        TEST_GENERATION,
    );
    let offer = CustodyOffer::new(&request, descriptor_set(), digest());
    let armed = arm_fixture(&pending, &request, &offer).unwrap();
    assert!(armed.is_valid());
    assert_eq!(armed.runtime, runtime);
    assert!(armed.arm(&request, &offer).is_none());
}

pub(super) fn request() -> CustodyRequest {
    let record = record();
    CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(&record),
        holder(),
        TEST_TOKEN.to_owned(),
        TEST_GENERATION,
    )
}

fn holder() -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: TEST_HOST_PID,
        start_time: TEST_HOST_START,
    }
}

pub(super) fn digest() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE)
}

pub(super) const fn descriptor_set() -> CustodyDescriptorSet {
    CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE)
}

fn record() -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START,
        child_pid: TEST_CHILD_PID,
        child_start_time: TEST_CHILD_START,
        boot_id: TEST_BOOT.to_owned(),
        socket_path: PathBuf::from(TEST_SOCKET),
        auth_token: TEST_TOKEN.to_owned(),
        controller_generation: TEST_GENERATION,
        started_at_ms: TEST_STARTED_AT_MS,
        command: TEST_SHELL.to_owned(),
        args: Vec::new(),
        cwd: PathBuf::from(TEST_DIRECTORY),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}

#[path = "record_commit_replay_tests.rs"]
mod commit_replay;
#[path = "../test_support/record_commit.rs"]
mod commit_support;
