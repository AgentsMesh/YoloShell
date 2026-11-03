use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity, CustodyOffer,
    CustodyRetirementEvidence, CustodyRuntimeIdentity,
};
use uuid::Uuid;

const AUTH_TOKEN: &str = "guardian-reboot-auth-token";
const SOURCE_BOOT: &str = "guardian-reboot-source";
const TARGET_BOOT: &str = "guardian-reboot-target";
const SOCKET_FILE: &str = "yolo-guardian-reboot.sock";
const WORKER_PID: u32 = 61;
const WORKER_START: u64 = 62;
const CHILD_PID: u32 = 63;
const CHILD_START: u64 = 64;
const GUARDIAN_PID: u32 = 65;
const GUARDIAN_START: u64 = 66;
const GENERATION: u64 = 1;
const STARTED_AT_MS: i64 = 1;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const LAST_OUTPUT_SEQUENCE: u64 = 1;
const IDENTITY_MAX_BYTES: u64 = 0;

#[test]
fn guardian_bound_armed_custody_round_trips_across_boots() {
    let runtime = runtime();
    let target = guarded_armed(&runtime);
    let source =
        source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_round_trip(&target, &source);
}

#[test]
fn guardian_bound_retired_receipt_round_trips_across_boots() {
    let runtime = runtime();
    let armed = guarded_armed(&runtime);
    let lease = armed.armed_lease().unwrap();
    let identity = file_identity();
    let evidence = CustodyRetirementEvidence::new(
        lease.controller_generation,
        identity,
        identity,
        identity,
        LAST_OUTPUT_SEQUENCE,
    );
    let target = armed.retire_with_evidence(lease, evidence).unwrap();
    let source =
        source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_eq!(source.retirement_evidence(), Some(evidence));
    assert_round_trip(&target, &source);
}

#[test]
fn guardian_preparation_round_trips_with_its_predecessor() {
    let runtime = runtime();
    let armed = guarded_armed(&runtime);
    let mut request = crate::pty_custody::tests::request();
    request.runtime = armed.runtime.clone();
    request.controller_generation += GENERATION;
    let offer = CustodyOffer::new(
        &request,
        crate::pty_custody::tests::descriptor_set(),
        crate::pty_custody::tests::digest(),
    );
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), guardian_process(), &request, &offer).unwrap();
    let target = armed.prepare_guardian(&request, &offer, guardian).unwrap();
    let source =
        source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_round_trip(&target, &source);
}

#[test]
fn reboot_rewrite_rejects_a_guardian_bound_to_another_boot() {
    let mut invalid = guarded_armed(&runtime());
    invalid.runtime.boot_id = SOURCE_BOOT.to_owned();
    assert!(!invalid.is_valid());
    assert!(rewrite_boot_only(&invalid, SOURCE_BOOT, TARGET_BOOT).is_err());
}

fn assert_round_trip(target: &CustodyDurableRecord, source: &CustodyDurableRecord) {
    assert!(source.is_valid());
    assert_eq!(source.runtime.boot_id, SOURCE_BOOT);
    let restored = rewrite_boot_only(source, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_eq!(restored, *target);
}

fn guarded_armed(runtime: &RuntimeRecord) -> CustodyDurableRecord {
    let pending = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(runtime));
    let mut request = crate::pty_custody::tests::request();
    request.runtime = pending.runtime.clone();
    request.controller_generation = runtime.controller_generation;
    let offer = CustodyOffer::new(
        &request,
        crate::pty_custody::tests::descriptor_set(),
        crate::pty_custody::tests::digest(),
    );
    crate::pty_custody::arm_fixture(&pending, &request, &offer).unwrap()
}

fn guardian_process() -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: GUARDIAN_PID,
        start_time: GUARDIAN_START,
    }
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

fn runtime() -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: TARGET_BOOT.to_owned(),
        socket_path: std::env::temp_dir().join(SOCKET_FILE),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: crate::agent::constants::FALLBACK_SHELL.to_owned(),
        args: Vec::new(),
        cwd: std::env::temp_dir(),
        cols: COLUMNS,
        rows: ROWS,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}
