use super::*;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use uuid::Uuid;

const AUTH_TOKEN: &str = "pty-custody-reboot-auth-token";
const SOURCE_BOOT: &str = "pty-custody-source-boot";
const TARGET_BOOT: &str = "pty-custody-target-boot";
const WORKER_PID: u32 = 51;
const WORKER_START: u64 = 52;
const CHILD_PID: u32 = 53;
const CHILD_START: u64 = 54;
const GENERATION: u64 = 1;
const STARTED_AT_MS: i64 = 1;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const PID_INCREMENT: u32 = 1;
const SOCKET_FILE: &str = "yolo-pty-custody-reboot.sock";
const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;

#[test]
fn reboot_migration_restores_only_runtime_boot_identity() {
    let runtime = runtime();
    let target =
        CustodyDurableRecord::pending(super::super::CustodyRuntimeIdentity::from_record(&runtime));
    let source =
        source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_eq!(source.runtime.boot_id, SOURCE_BOOT);
}

#[test]
fn reboot_migration_rejects_unrelated_runtime_mutation() {
    let runtime = runtime();
    let mut target =
        CustodyDurableRecord::pending(super::super::CustodyRuntimeIdentity::from_record(&runtime));
    target.runtime.worker_pid += PID_INCREMENT;
    assert!(source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).is_err());
}

#[test]
fn reboot_migration_accepts_promoted_successor_custody_generation() {
    let runtime = runtime();
    let pending =
        CustodyDurableRecord::pending(super::super::CustodyRuntimeIdentity::from_record(&runtime));
    let mut request = super::super::tests::request();
    request.runtime = pending.runtime.clone();
    request.controller_generation = runtime.controller_generation;
    let first = super::super::CustodyOffer::new(
        &request,
        super::super::tests::descriptor_set(),
        super::super::tests::digest(),
    );
    let armed = crate::pty_custody::arm_fixture(&pending, &request, &first).unwrap();
    request.controller_generation += SUCCESSOR_GENERATION_INCREMENT;
    request.lease_id = super::super::CustodyLeaseId::new();
    let successor = super::super::CustodyOffer::new(
        &request,
        super::super::CustodyDescriptorSet::ReleasedSuccessor,
        super::super::tests::digest(),
    );
    let target = armed.rearm(&request, &successor).unwrap();
    assert!(source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT,).is_ok());
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
