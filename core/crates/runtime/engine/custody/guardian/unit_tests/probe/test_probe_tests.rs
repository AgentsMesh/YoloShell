use super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};

const AUTH_TOKEN: &str = "guardian-probe-test-token";
const BOOT_ID: &str = "guardian-probe-test-boot";
const NAMESPACE_LABEL: &str = "guardian probe test session";
const CONTROLLER_GENERATION: u64 = 17;
const WORKER_PID: u32 = 72_001;
const WORKER_START_TIME: u64 = 82_001;
const CHILD_PID: u32 = 72_002;
const CHILD_START_TIME: u64 = 82_002;
const HOST_PID: u32 = 72_003;
const HOST_START_TIME: u64 = 82_003;
const GUARDIAN_PID: u32 = 72_004;
const GUARDIAN_START_TIME: u64 = 82_004;
const READER_HANDLE: u64 = 92_001;
const WRITER_HANDLE: u64 = 92_002;
const DIGEST_BYTE: u8 = 0x73;
const IDENTITY_MAX_BYTES: u64 = 64;
const FIELD_INCREMENT: u64 = 1;
const SCHEMA_INCREMENT: u16 = 1;

#[test]
fn probe_path_is_scoped_to_the_exact_guardian_id() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let terminal_id = Uuid::new_v4();
    let first = path(&paths, terminal_id, Uuid::new_v4());
    let second = path(&paths, terminal_id, Uuid::new_v4());

    assert_ne!(first, second);
    assert_eq!(
        first.parent(),
        Some(probe_directory(&paths, terminal_id).as_path())
    );
    assert_eq!(second.parent(), first.parent());
}

#[test]
fn binding_rejects_each_guardian_and_durable_aba_dimension() {
    let record = armed_record();
    let (identity, replacement_identity) = distinct_file_identities();
    let exact = ProbeBinding::capture(&record, identity).unwrap();
    let guardian = record.armed_lease().unwrap().guardian.unwrap();
    assert!(exact.matches(&record, identity).is_some());
    assert!(exact.matches_guardian(guardian.guardian_id(), guardian.process()));

    let mut stale = exact;
    stale.schema_version = stale.schema_version.saturating_add(SCHEMA_INCREMENT);
    assert!(stale.matches(&record, identity).is_none());
    stale = exact;
    stale.guardian_id = Uuid::new_v4();
    assert!(stale.matches(&record, identity).is_none());
    stale = exact;
    stale.guardian_process.pid = stale
        .guardian_process
        .pid
        .saturating_add(FIELD_INCREMENT as u32);
    assert!(stale.matches(&record, identity).is_none());
    stale = exact;
    stale.guardian_process.start_time = stale
        .guardian_process
        .start_time
        .saturating_add(FIELD_INCREMENT);
    assert!(stale.matches(&record, identity).is_none());
    stale = exact;
    stale.controller_generation = stale.controller_generation.saturating_add(FIELD_INCREMENT);
    assert!(stale.matches(&record, identity).is_none());
    assert!(exact.matches(&record, replacement_identity).is_none());
}

#[test]
fn stale_successor_cannot_publish_for_the_durable_guardian() {
    let record = armed_record();
    let (identity, _) = distinct_file_identities();
    let exact = ProbeBinding::capture(&record, identity).unwrap();
    let guardian = record.armed_lease().unwrap().guardian.unwrap();

    assert!(!exact.matches_guardian(Uuid::new_v4(), guardian.process()));
    assert!(
        !exact.matches_guardian(
            guardian.guardian_id(),
            holder(
                guardian
                    .process()
                    .pid
                    .saturating_add(FIELD_INCREMENT as u32),
                guardian
                    .process()
                    .start_time
                    .saturating_add(FIELD_INCREMENT),
            ),
        )
    );
}

#[test]
fn durable_reread_rejects_same_value_record_replacement() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let record = armed_record();
    paths.ensure_session(record.runtime.terminal_id).unwrap();
    let session_dir = paths.session_dir(record.runtime.terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session_dir)]).unwrap();
    let durable_path = paths.pty_custody(record.runtime.terminal_id);
    namespace.write_json(&durable_path, &record).unwrap();
    let original = namespace
        .read_json_with_identity::<CustodyDurableRecord>(&durable_path)
        .unwrap()
        .1;
    confirm_durable_unchanged(&namespace, &paths, &record, original).unwrap();

    namespace.write_json(&durable_path, &record).unwrap();
    let replacement = namespace
        .read_json_with_identity::<CustodyDurableRecord>(&durable_path)
        .unwrap()
        .1;
    assert_ne!(replacement, original);
    assert!(confirm_durable_unchanged(&namespace, &paths, &record, original).is_err());
}

fn armed_record() -> CustodyDurableRecord {
    let runtime = CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START_TIME,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START_TIME,
        boot_id: BOOT_ID.to_owned(),
    };
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
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
    let guardian = CustodyGuardianBinding::new(
        Uuid::new_v4(),
        holder(GUARDIAN_PID, GUARDIAN_START_TIME),
        &request,
        &offer,
    )
    .unwrap();
    CustodyDurableRecord::pending(runtime)
        .prepare_guardian(&request, &offer, guardian)
        .unwrap()
        .arm_prepared_guardian(&request, &offer, guardian)
        .unwrap()
}

fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}

fn distinct_file_identities() -> (FileIdentity, FileIdentity) {
    let first = tempfile::NamedTempFile::new().unwrap();
    let second = tempfile::NamedTempFile::new().unwrap();
    (file_identity(first.path()), file_identity(second.path()))
}

fn file_identity(path: &std::path::Path) -> FileIdentity {
    FileIdentity::inspect_bounded(&path.metadata().unwrap(), path, IDENTITY_MAX_BYTES).unwrap()
}
