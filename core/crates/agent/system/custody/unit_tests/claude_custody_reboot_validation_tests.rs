use super::*;
use crate::claude_custody::types::{CustodyPublication, CustodyRecord, CustodyState};
use crate::model::{BootstrapDelivery, RuntimeRecord};
use uuid::Uuid;

const AUTH_TOKEN: &str = "claude-custody-reboot-auth-token";
const SOURCE_BOOT: &str = "claude-custody-source-boot";
const TARGET_BOOT: &str = "claude-custody-target-boot";
const HISTORICAL_BOOT: &str = "claude-custody-historical-boot";
const UNKNOWN_BOOT: &str = "unknown:claude-custody-boot";
const WORKER_PID: u32 = 41;
const WORKER_START: u64 = 42;
const CHILD_PID: u32 = 43;
const CHILD_START: u64 = 44;
const FIRST_GUARDIAN_PID: u32 = 45;
const SECOND_GUARDIAN_PID: u32 = 46;
const IDENTITY_START_OFFSET: u64 = 100;
const GENERATION: u64 = 1;
const STARTED_AT_MS: i64 = 1;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const FIRST_RECORD_INDEX: usize = 0;
const PRIMARY_GUARDIAN_INDEX: usize = 0;
const SOCKET_FILE: &str = "yolo-claude-custody-reboot.sock";
const WORKER_PID_INCREMENT: u32 = 1;

#[test]
fn related_custody_restores_only_target_boot_identities() {
    let runtime = runtime();
    let target = ledger(TARGET_BOOT, WORKER_PID, WORKER_START);
    let source =
        source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert!(
        identities(&source.records[FIRST_RECORD_INDEX].spec)
            .all(|identity| identity.boot_id == SOURCE_BOOT)
    );
}

#[test]
fn empty_ledger_remains_valid_when_no_boot_identity_requires_migration() {
    let target = CustodyLedger {
        schema_version: super::super::types::LEDGER_SCHEMA_VERSION,
        records: Vec::new(),
    };
    let source =
        source_before_reboot_migration(&target, &runtime(), SOURCE_BOOT, TARGET_BOOT).unwrap();
    assert_eq!(source, target);
}

#[test]
fn historical_only_and_mixed_ledgers_fail_closed() {
    let runtime = runtime();
    let historical = ledger(HISTORICAL_BOOT, WORKER_PID, WORKER_START);
    assert!(
        source_before_reboot_migration(&historical, &runtime, SOURCE_BOOT, TARGET_BOOT,).is_err()
    );
    let mut mixed = ledger(TARGET_BOOT, WORKER_PID, WORKER_START);
    mixed.records.extend(historical.records);
    assert!(source_before_reboot_migration(&mixed, &runtime, SOURCE_BOOT, TARGET_BOOT).is_err());
}

#[test]
fn partial_boot_and_foreign_worker_identities_fail_closed() {
    let runtime = runtime();
    let mut target = ledger(TARGET_BOOT, WORKER_PID + WORKER_PID_INCREMENT, WORKER_START);
    assert!(source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).is_err());
    target = ledger(TARGET_BOOT, WORKER_PID, WORKER_START);
    target.records[FIRST_RECORD_INDEX].spec.guardians[PRIMARY_GUARDIAN_INDEX].boot_id =
        HISTORICAL_BOOT.to_owned();
    assert!(source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).is_err());
    target.records[FIRST_RECORD_INDEX].spec.guardians[PRIMARY_GUARDIAN_INDEX].boot_id =
        UNKNOWN_BOOT.to_owned();
    assert!(source_before_reboot_migration(&target, &runtime, SOURCE_BOOT, TARGET_BOOT).is_err());
}

fn ledger(boot: &str, worker_pid: u32, worker_start: u64) -> CustodyLedger {
    CustodyLedger {
        schema_version: super::super::types::LEDGER_SCHEMA_VERSION,
        records: vec![record(boot, worker_pid, worker_start)],
    }
}

fn record(boot: &str, worker_pid: u32, worker_start: u64) -> CustodyRecord {
    let identity = |pid: u32| DurableProcessIdentity {
        boot_id: boot.to_owned(),
        pid,
        start_time: IDENTITY_START_OFFSET + u64::from(pid),
    };
    CustodyRecord {
        spec: CustodySpec {
            invocation_id: Uuid::new_v4(),
            target: identity(CHILD_PID),
            worker: DurableProcessIdentity {
                boot_id: boot.to_owned(),
                pid: worker_pid,
                start_time: worker_start,
            },
            guardians: [identity(FIRST_GUARDIAN_PID), identity(SECOND_GUARDIAN_PID)],
        },
        state: CustodyState::Containing,
        publication: CustodyPublication::Active,
    }
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
