use std::path::PathBuf;
use uuid::Uuid;
use yolo_terminal_model::{BootstrapDelivery, RuntimeRecord, RuntimeUpdateError};

const SCHEMA_VERSION: u16 = 2;
const TERMINAL_ID: Uuid = Uuid::from_u128(1);
const RUNTIME_EPOCH: Uuid = Uuid::from_u128(2);
const WORKER_ID: Uuid = Uuid::from_u128(3);
const OTHER_ID: Uuid = Uuid::from_u128(4);
const WORKER_PID: u32 = 101;
const CHILD_PID: u32 = 102;
const START_TIME: u64 = 201;
const OTHER_START_TIME: u64 = 202;
const INITIAL_GENERATION: u64 = 0;
const STARTED_AT_MS: i64 = 0;
const BOOT_ID: &str = "test-boot";
const AUTH_TOKEN: &str = "test-token";
const SOCKET_PATH: &str = "/test/socket";
const SHELL_PATH: &str = "/bin/zsh";
const WORKING_DIRECTORY: &str = "/test/workspace";
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;

#[test]
fn only_proven_bootstrap_deliveries_are_ready() {
    for delivery in [
        BootstrapDelivery::Pending,
        BootstrapDelivery::Survivor,
        BootstrapDelivery::Failed,
        BootstrapDelivery::Unknown,
    ] {
        assert!(!delivery.is_ready(), "{delivery:?}");
    }
    for delivery in [BootstrapDelivery::NotRequired, BootstrapDelivery::Delivered] {
        assert!(delivery.is_ready(), "{delivery:?}");
    }
}

#[test]
fn ready_and_survivor_deliveries_are_serviceable() {
    for delivery in [
        BootstrapDelivery::Pending,
        BootstrapDelivery::Failed,
        BootstrapDelivery::Unknown,
    ] {
        assert!(!delivery.is_serviceable(), "{delivery:?}");
    }
    for delivery in [
        BootstrapDelivery::NotRequired,
        BootstrapDelivery::Delivered,
        BootstrapDelivery::Survivor,
    ] {
        assert!(delivery.is_serviceable(), "{delivery:?}");
    }
}

#[test]
fn process_identity_rejects_every_custody_field_mismatch() {
    let record = runtime_record();
    assert!(record.has_same_process_identity(&record.clone()));

    let mutations: [fn(&mut RuntimeRecord); 4] = [
        |value| value.schema_version += 1,
        |value| value.terminal_id = OTHER_ID,
        |value| value.runtime_epoch = OTHER_ID,
        |value| value.worker_id = OTHER_ID,
    ];
    let mut candidates = mutations
        .into_iter()
        .map(|mutate| {
            let mut candidate = record.clone();
            mutate(&mut candidate);
            candidate
        })
        .collect::<Vec<_>>();
    candidates.extend(process_mismatches(&record));
    for candidate in candidates {
        assert!(!record.has_same_process_identity(&candidate));
    }
}
#[test]
fn update_contract_rejects_launch_binding_changes() {
    let current = runtime_record();
    let mutations: [fn(&mut RuntimeRecord); 4] = [
        |value| value.started_at_ms += 1,
        |value| value.command.push_str("-changed"),
        |value| value.args.push("changed-argument".into()),
        |value| value.cwd.push("changed-directory"),
    ];
    for mutate in mutations {
        let mut candidate = current.clone();
        mutate(&mut candidate);
        assert_eq!(
            candidate.validate_update_from(&current),
            Err(RuntimeUpdateError::LaunchIdentity)
        );
    }
}
#[test]
fn update_contract_allows_monotonic_generation_and_normalized_resize() {
    let mut current = runtime_record();
    current.controller_generation = INITIAL_GENERATION + 1;
    for generation in [INITIAL_GENERATION + 1, INITIAL_GENERATION + 2] {
        let mut candidate = current.clone();
        candidate.controller_generation = generation;
        candidate.cols = TERMINAL_COLUMNS + 1;
        candidate.rows = TERMINAL_ROWS + 1;
        assert_eq!(candidate.validate_update_from(&current), Ok(()));
    }
    let mut regressed = current.clone();
    regressed.controller_generation = INITIAL_GENERATION;
    assert_eq!(
        regressed.validate_update_from(&current),
        Err(RuntimeUpdateError::ControllerGenerationRegression)
    );
    let mut malformed = current.clone();
    malformed.cols = 0;
    assert_eq!(
        malformed.validate_update_from(&current),
        Err(RuntimeUpdateError::NonCanonicalTerminalDimensions)
    );
    malformed.cols = yolo_terminal_model::MIN_TERMINAL_COLUMNS;
    assert_eq!(malformed.validate_update_from(&current), Ok(()));
}
#[test]
fn update_contract_enforces_bootstrap_delivery_terminal_states() {
    let current = runtime_record();
    let terminal = [
        BootstrapDelivery::NotRequired,
        BootstrapDelivery::Delivered,
        BootstrapDelivery::Survivor,
        BootstrapDelivery::Failed,
        BootstrapDelivery::Unknown,
    ];
    for delivery in terminal {
        let mut from_pending = current.clone();
        from_pending.bootstrap_delivery = BootstrapDelivery::Pending;
        let mut candidate = from_pending.clone();
        candidate.bootstrap_delivery = delivery;
        assert_eq!(candidate.validate_update_from(&from_pending), Ok(()));

        let mut same = current.clone();
        same.bootstrap_delivery = delivery;
        assert_eq!(same.validate_update_from(&same), Ok(()));
        for replacement in terminal {
            if replacement == delivery {
                continue;
            }
            let mut changed = same.clone();
            changed.bootstrap_delivery = replacement;
            assert_eq!(
                changed.validate_update_from(&same),
                Err(RuntimeUpdateError::BootstrapDeliveryTransition)
            );
        }
    }
}
fn process_mismatches(record: &RuntimeRecord) -> Vec<RuntimeRecord> {
    let mut values = Vec::new();
    macro_rules! mismatch {
        ($field:ident, $value:expr) => {{
            let mut candidate = record.clone();
            candidate.$field = $value;
            values.push(candidate);
        }};
    }
    mismatch!(worker_pid, WORKER_PID + 1);
    mismatch!(worker_start_time, OTHER_START_TIME);
    mismatch!(child_pid, CHILD_PID + 1);
    mismatch!(child_start_time, OTHER_START_TIME);
    mismatch!(boot_id, format!("{BOOT_ID}-other"));
    mismatch!(socket_path, PathBuf::from(format!("{SOCKET_PATH}-other")));
    mismatch!(auth_token, format!("{AUTH_TOKEN}-other"));
    values
}

fn runtime_record() -> RuntimeRecord {
    RuntimeRecord {
        schema_version: SCHEMA_VERSION,
        terminal_id: TERMINAL_ID,
        runtime_epoch: RUNTIME_EPOCH,
        worker_id: WORKER_ID,
        worker_pid: WORKER_PID,
        worker_start_time: START_TIME,
        child_pid: CHILD_PID,
        child_start_time: START_TIME,
        boot_id: BOOT_ID.to_owned(),
        socket_path: SOCKET_PATH.into(),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: INITIAL_GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: SHELL_PATH.to_owned(),
        args: Vec::new(),
        cwd: WORKING_DIRECTORY.into(),
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}
