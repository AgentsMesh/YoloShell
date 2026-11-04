use super::super::{BootstrapDelivery, RuntimeView};
use crate::TerminalRuntimeStatus;
use uuid::Uuid;

const WORKER_PID: u32 = 101;
const CHILD_PID: u32 = 102;
const TEST_PROCESS_START_TIME: u64 = 1_000;
const OUTPUT_SEQUENCE: u64 = 0;
const INVALID_CAPABILITY_SCALAR: &str = "invalid";
const INVALID_CAPABILITY_NUMBER: u8 = 1;

#[test]
fn capability_wire_distinguishes_missing_empty_and_null() {
    let view = |capabilities| RuntimeView {
        presentation: Default::default(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: TEST_PROCESS_START_TIME,
        child_pid: CHILD_PID,
        child_start_time: TEST_PROCESS_START_TIME,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: OUTPUT_SEQUENCE,
        exit: None,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
        capabilities,
        fault_reap: None,
    };
    let unknown = serde_json::to_value(view(None)).unwrap();
    assert!(unknown.get(stringify!(capabilities)).is_none());
    let decoded: RuntimeView = serde_json::from_value(unknown).unwrap();
    assert_eq!(decoded.capabilities, None);
    assert_eq!(decoded.fault_reap, None);

    let mut known = serde_json::to_value(view(Some(Vec::new()))).unwrap();
    assert_eq!(known[stringify!(capabilities)], serde_json::json!([]));
    let decoded: RuntimeView = serde_json::from_value(known.clone()).unwrap();
    assert_eq!(decoded.capabilities, Some(Vec::new()));

    known[stringify!(capabilities)] = serde_json::Value::Null;
    assert!(serde_json::from_value::<RuntimeView>(known).is_err());
}

#[test]
fn capability_wire_rejects_non_array_and_non_string_values() {
    for capabilities in [
        serde_json::json!(INVALID_CAPABILITY_SCALAR),
        serde_json::json!([INVALID_CAPABILITY_NUMBER]),
    ] {
        let mut value = serde_json::to_value(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: WORKER_PID,
            worker_start_time: TEST_PROCESS_START_TIME,
            child_pid: CHILD_PID,
            child_start_time: TEST_PROCESS_START_TIME,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        })
        .unwrap();
        value[stringify!(capabilities)] = capabilities;
        assert!(serde_json::from_value::<RuntimeView>(value).is_err());
    }
}

#[test]
fn fault_reap_wire_rejects_explicit_null() {
    let mut value = serde_json::to_value(RuntimeView {
        presentation: Default::default(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: TEST_PROCESS_START_TIME,
        child_pid: CHILD_PID,
        child_start_time: TEST_PROCESS_START_TIME,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: OUTPUT_SEQUENCE,
        exit: None,
        bootstrap_delivery: BootstrapDelivery::NotRequired,
        capabilities: Some(Vec::new()),
        fault_reap: None,
    })
    .unwrap();
    value[stringify!(fault_reap)] = serde_json::Value::Null;
    assert!(serde_json::from_value::<RuntimeView>(value).is_err());
}
