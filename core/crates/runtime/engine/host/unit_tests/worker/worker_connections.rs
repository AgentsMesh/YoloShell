use super::*;

const TEST_SCHEMA_VERSION: u16 = 1;
const TEST_CONTROLLER_GENERATION: u64 = 7;
const TEST_NEXT_CONTROLLER_GENERATION: u64 = 8;
const TEST_WORKER_PID: u32 = 1;
const TEST_WORKER_START_TIME: u64 = 2;
const TEST_CHILD_PID: u32 = 3;
const TEST_CHILD_START_TIME: u64 = 4;
const TEST_STARTED_AT_MS: i64 = 5;
const TEST_COLUMNS: u16 = 80;
const TEST_ROWS: u16 = 24;

#[test]
fn runtime_epoch_and_process_identity_are_required_for_reuse() {
    let cached = record(Uuid::new_v4(), TEST_CONTROLLER_GENERATION);
    let hello = WorkerHello {
        runtime: cached.clone(),
        capabilities: Vec::new(),
    };
    assert!(identity_matches_record(&cached, &hello, &cached));
    assert!(!identity_matches_record(
        &cached,
        &hello,
        &record(Uuid::new_v4(), TEST_CONTROLLER_GENERATION)
    ));
    assert!(!identity_matches_record(
        &cached,
        &hello,
        &RuntimeRecord {
            controller_generation: TEST_NEXT_CONTROLLER_GENERATION,
            ..cached.clone()
        }
    ));
}

fn record(runtime_epoch: Uuid, controller_generation: u64) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: TEST_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch,
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START_TIME,
        child_pid: TEST_CHILD_PID,
        child_start_time: TEST_CHILD_START_TIME,
        boot_id: "boot".into(),
        socket_path: "/tmp/worker.sock".into(),
        auth_token: "token".into(),
        controller_generation,
        started_at_ms: TEST_STARTED_AT_MS,
        command: "shell".into(),
        args: Vec::new(),
        cwd: "/tmp".into(),
        cols: TEST_COLUMNS,
        rows: TEST_ROWS,
        bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
    }
}
