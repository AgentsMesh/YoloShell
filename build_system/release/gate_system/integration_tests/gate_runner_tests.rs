use super::support::{COMPLETION_SECONDS, REVISION, coverage_output};
use super::test_constants::{BAZEL_OUTPUT_REVISION_A, PINNED_BAZEL_TEST_PATH, RUST_COVERAGE_BYTES};
use crate::gate_runner::{ProcessExecuting, run_gate};
use crate::requirements::REQUIRED_GATES;
use crate::snapshot::GateSnapshot;
use anyhow::{Result, bail};
use std::cell::RefCell;
use std::path::PathBuf;
use yolo_release_contracts::{BAZEL_BATCH_ARGUMENT, RUST_COVERAGE_GATE, UNIT_GATE};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::TrustedBazel;

struct RecordingExecutor {
    commands: RefCell<Vec<ProcessCommand>>,
    fail: bool,
}

impl ProcessExecuting for RecordingExecutor {
    fn execute(&self, command: &ProcessCommand) -> Result<()> {
        self.commands.borrow_mut().push(command.clone());
        if self.fail {
            bail!("forced gate failure");
        }
        Ok(())
    }

    fn completed_at_unix_seconds(&self) -> Result<u64> {
        Ok(COMPLETION_SECONDS)
    }
}

#[test]
fn gate_evidence_comes_from_exact_executed_batch_commands() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let launcher = TrustedBazel::fixture(PathBuf::from(PINNED_BAZEL_TEST_PATH));
    let executor = RecordingExecutor {
        commands: RefCell::new(Vec::new()),
        fail: false,
    };
    let requirement = REQUIRED_GATES
        .into_iter()
        .find(|gate| gate.name == UNIT_GATE)
        .unwrap();
    let evidence = run_gate(requirement, REVISION, &snapshot, &launcher, &executor).unwrap();
    let commands = executor.commands.borrow();
    assert_eq!(commands.len(), requirement.bazel_commands().unwrap().len());
    assert_eq!(evidence.commands[0][1], BAZEL_BATCH_ARGUMENT);
    assert_eq!(
        evidence.commands,
        commands
            .iter()
            .map(ProcessCommand::evidence)
            .collect::<Vec<_>>()
    );
    assert_eq!(evidence.completed_at_unix_seconds, COMPLETION_SECONDS);
    assert_eq!(evidence.source_revision, REVISION);
}

#[test]
fn failed_command_cannot_produce_gate_evidence() {
    let snapshot = GateSnapshot::fixture().unwrap();
    let launcher = TrustedBazel::fixture(PathBuf::from(PINNED_BAZEL_TEST_PATH));
    let executor = RecordingExecutor {
        commands: RefCell::new(Vec::new()),
        fail: true,
    };
    let requirement = REQUIRED_GATES
        .into_iter()
        .find(|requirement| requirement.bazel_commands().is_some())
        .unwrap();
    assert!(run_gate(requirement, REVISION, &snapshot, &launcher, &executor).is_err());
}

#[test]
fn coverage_gate_consumes_only_the_pinned_report() {
    let snapshot = GateSnapshot::fixture().unwrap();
    coverage_output(&snapshot, BAZEL_OUTPUT_REVISION_A, RUST_COVERAGE_BYTES);
    let launcher = TrustedBazel::fixture(PathBuf::from(PINNED_BAZEL_TEST_PATH));
    let executor = RecordingExecutor {
        commands: RefCell::new(Vec::new()),
        fail: false,
    };
    let requirement = REQUIRED_GATES
        .into_iter()
        .find(|gate| gate.name == RUST_COVERAGE_GATE)
        .unwrap();
    let evidence = run_gate(requirement, REVISION, &snapshot, &launcher, &executor).unwrap();
    assert_eq!(
        evidence.commands.len(),
        requirement.bazel_commands().unwrap().len()
    );
    assert_eq!(
        evidence.commands,
        requirement
            .command_evidence(&evidence.tool, &snapshot.execution().unwrap())
            .unwrap()
    );
}
