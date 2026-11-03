use super::wire::{ObservedProcessWire, ProcessSnapshotEnvelope, ProcessSnapshotWireOutcome};
use super::{PROCESS_SNAPSHOT_SCHEMA_VERSION, ProcessSnapshotOutcome, ProcessSnapshotReceipt};
use crate::process::ProcessIdentity;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) const TEST_ROOT_PID: u32 = 41;
pub(super) const TEST_ROOT_START_TIME: u64 = 73;
pub(super) const TEST_STARTED_AT_MS: i64 = 1_001;
pub(super) const TEST_COMPLETED_AT_MS: i64 = 1_002;
pub(super) const TEST_FAILURE: &str = "scan unavailable";
const TEST_PARENT_PID: u32 = 29;
const TEST_PROCESS_GROUP: i32 = 17;
const TEST_EXECUTABLE: &str = "/usr/bin/agent";
const TEST_ENVIRONMENT_KEY: &str = "YOLO_TERMINAL_ID";
const TEST_ENVIRONMENT_VALUE: &str = "session";
const TEST_CWD: &str = "/tmp/project";
const TEST_EVIDENCE: &str = "/tmp/evidence.jsonl";

pub(super) fn environment_keys() -> Vec<String> {
    vec![TEST_ENVIRONMENT_KEY.to_owned()]
}

pub(super) fn root() -> ProcessIdentity {
    ProcessIdentity {
        pid: TEST_ROOT_PID,
        start_time: TEST_ROOT_START_TIME,
    }
}

pub(super) fn observed_wire() -> ObservedProcessWire {
    ObservedProcessWire {
        pid: TEST_ROOT_PID,
        start_time: TEST_ROOT_START_TIME,
        parent_pid: TEST_PARENT_PID,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_PROCESS_GROUP,
        executable: TEST_EXECUTABLE.to_owned(),
        argv: vec![
            TEST_EXECUTABLE.to_owned(),
            crate::agent::codex::constants::RESUME_ARGUMENT.to_owned(),
        ],
        environment: Some(BTreeMap::from([(
            TEST_ENVIRONMENT_KEY.to_owned(),
            TEST_ENVIRONMENT_VALUE.to_owned(),
        )])),
        cwd: Some(PathBuf::from(TEST_CWD).into()),
        evidence_paths: Some(vec![PathBuf::from(TEST_EVIDENCE).into()]),
    }
}

pub(super) fn envelope(outcome: ProcessSnapshotWireOutcome) -> ProcessSnapshotEnvelope {
    ProcessSnapshotEnvelope {
        schema_version: PROCESS_SNAPSHOT_SCHEMA_VERSION,
        root_pid: TEST_ROOT_PID,
        root_start_time: TEST_ROOT_START_TIME,
        started_at_ms: TEST_STARTED_AT_MS,
        completed_at_ms: TEST_COMPLETED_AT_MS,
        outcome,
    }
}

pub(super) fn frame(outcome: ProcessSnapshotWireOutcome) -> Vec<u8> {
    crate::protocol::encode(&envelope(outcome)).unwrap()
}

pub(super) fn receipt(outcome: ProcessSnapshotOutcome) -> ProcessSnapshotReceipt {
    ProcessSnapshotReceipt {
        outcome,
        started_at_ms: TEST_STARTED_AT_MS,
        completed_at_ms: TEST_COMPLETED_AT_MS,
    }
}
