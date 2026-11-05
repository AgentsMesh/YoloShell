use super::super::constants::{
    MAX_PROCESS_ARGV_BYTES, MAX_PROCESS_ENVIRONMENT_KEYS, MAX_PROCESS_ENVIRONMENT_VALUE_BYTES,
    MAX_PROCESS_EVIDENCE_PATHS, MAX_SINGLE_PROCESS_ARGV_BYTES,
};
use super::validation::decode_processes;
use super::wire::ObservedProcessWire;
use std::collections::BTreeMap;
use std::path::PathBuf;

const TEST_PID: u32 = 101;
const TEST_START_TIME: u64 = 307;
const TEST_PARENT_PID: u32 = 89;
const TEST_PROCESS_GROUP: i32 = 53;
const TEST_EXECUTABLE: &str = "/usr/bin/codex";
const TEST_CWD: &str = "/tmp/project";
const TEST_ENVIRONMENT_KEY: &str = "YOLO_TERMINAL_ID";
const TEST_ENVIRONMENT_VALUE: &str = "session";
const TEST_ENVIRONMENT_KEY_PREFIX: &str = "KEY_";
const TEST_EVIDENCE_PREFIX: &str = "/tmp/evidence-";
const TEST_RELATIVE_PATH: &str = "relative/path";
const TEST_FILL: &str = "x";
const LIMIT_EXCESS: usize = 1;
const EXPECTED_PROCESS_COUNT: usize = 1;

fn valid_process(pid: u32) -> ObservedProcessWire {
    ObservedProcessWire {
        pid,
        start_time: TEST_START_TIME,
        parent_pid: TEST_PARENT_PID,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_PROCESS_GROUP,
        executable: TEST_EXECUTABLE.to_owned(),
        argv: vec![TEST_EXECUTABLE.to_owned()],
        environment: Some(BTreeMap::from([(
            TEST_ENVIRONMENT_KEY.to_owned(),
            TEST_ENVIRONMENT_VALUE.to_owned(),
        )])),
        cwd: Some(PathBuf::from(TEST_CWD).into()),
        evidence_paths: None,
    }
}

#[test]
fn stable_snapshot_rejects_duplicate_or_invalid_process_identity() {
    assert!(decode_processes(vec![valid_process(TEST_PID), valid_process(TEST_PID)], &super::test_support::environment_keys()).is_err());
    for pid in [u32::default(), u32::MAX] {
        assert!(decode_processes(vec![valid_process(pid)], &super::test_support::environment_keys()).is_err());
    }

    let mut invalid_pid = valid_process(TEST_PID);
    invalid_pid.start_time = u64::default();
    assert!(decode_processes(vec![invalid_pid], &super::test_support::environment_keys()).is_err());
}

#[test]
fn stable_snapshot_rejects_relative_executable_cwd_and_evidence_paths() {
    let mut process = valid_process(TEST_PID);
    process.executable = TEST_RELATIVE_PATH.to_owned();
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());

    let mut process = valid_process(TEST_PID);
    process.cwd = Some(PathBuf::from(TEST_RELATIVE_PATH).into());
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());

    let mut process = valid_process(TEST_PID);
    process.evidence_paths = Some(vec![PathBuf::from(TEST_RELATIVE_PATH).into()]);
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());
}

#[test]
fn stable_snapshot_rejects_argv_environment_and_evidence_over_budget() {
    let mut process = valid_process(TEST_PID);
    process.argv = vec![TEST_FILL.repeat(MAX_SINGLE_PROCESS_ARGV_BYTES + LIMIT_EXCESS)];
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());

    let mut process = valid_process(TEST_PID);
    process.environment = Some(BTreeMap::from([(
        TEST_ENVIRONMENT_KEY.to_owned(),
        TEST_FILL.repeat(MAX_PROCESS_ENVIRONMENT_VALUE_BYTES + LIMIT_EXCESS),
    )]));
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());

    let mut process = valid_process(TEST_PID);
    process.environment = Some(
        (0..=MAX_PROCESS_ENVIRONMENT_KEYS)
            .map(|index| {
                (
                    format!("{TEST_ENVIRONMENT_KEY_PREFIX}{index}"),
                    TEST_ENVIRONMENT_VALUE.to_owned(),
                )
            })
            .collect(),
    );
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());

    let mut process = valid_process(TEST_PID);
    process.evidence_paths = Some(
        (0..=MAX_PROCESS_EVIDENCE_PATHS)
            .map(|index| PathBuf::from(format!("{TEST_EVIDENCE_PREFIX}{index}")).into())
            .collect(),
    );
    assert!(decode_processes(vec![process], &super::test_support::environment_keys()).is_err());
}

#[test]
fn stable_snapshot_combines_argv_and_environment_budgets() {
    let environment_bytes = TEST_ENVIRONMENT_KEY.len() + TEST_ENVIRONMENT_VALUE.len();
    let mut exact = valid_process(TEST_PID);
    exact.argv = vec![TEST_FILL.repeat(MAX_SINGLE_PROCESS_ARGV_BYTES - environment_bytes)];
    assert!(decode_processes(vec![exact], &super::test_support::environment_keys()).is_ok());

    let mut excessive = valid_process(TEST_PID);
    excessive.argv =
        vec![TEST_FILL.repeat(MAX_SINGLE_PROCESS_ARGV_BYTES - environment_bytes + LIMIT_EXCESS)];
    assert!(decode_processes(vec![excessive], &super::test_support::environment_keys()).is_err());
}

#[test]
fn stable_snapshot_applies_combined_budget_across_processes() {
    let environment_bytes = TEST_ENVIRONMENT_KEY.len() + TEST_ENVIRONMENT_VALUE.len();
    let process_count = MAX_PROCESS_ARGV_BYTES / MAX_SINGLE_PROCESS_ARGV_BYTES;
    let full_budget = || {
        (0..process_count)
            .map(|index| {
                let mut process = valid_process(TEST_PID + u32::try_from(index).unwrap());
                process.argv =
                    vec![TEST_FILL.repeat(MAX_SINGLE_PROCESS_ARGV_BYTES - environment_bytes)];
                process
            })
            .collect::<Vec<_>>()
    };
    assert!(decode_processes(full_budget(), &super::test_support::environment_keys()).is_ok());

    let mut processes = full_budget();
    processes.push(valid_process(
        TEST_PID + u32::try_from(process_count).unwrap(),
    ));
    assert!(decode_processes(processes, &super::test_support::environment_keys()).is_err());
}

#[test]
fn stable_snapshot_accepts_incomplete_optional_observations() {
    let mut process = valid_process(TEST_PID);
    process.argv.clear();
    process.environment = None;
    process.cwd = None;
    process.evidence_paths = None;

    let decoded = decode_processes(vec![process], &super::test_support::environment_keys()).unwrap();

    assert_eq!(decoded.len(), EXPECTED_PROCESS_COUNT);
    assert!(decoded[0].argv.is_empty());
    assert!(decoded[0].environment.is_none());
}

#[path = "process_snapshot_validation_adversarial_tests.rs"]
mod adversarial;
#[path = "process_snapshot_global_budget_tests.rs"]
mod global_budget;
