use super::support::{execution, gates, launcher};
use crate::requirements::{PACKAGE_COMMAND, REQUIRED_GATES, coverage_report_path};
use yolo_release_contracts::{
    BAZEL_BATCH_ARGUMENT, BAZEL_MACOS_CONFIG_ARGUMENT, BAZEL_NO_HOME_RC_ARGUMENT,
    BAZEL_NO_SYSTEM_RC_ARGUMENT, BAZEL_NO_WORKSPACE_RC_ARGUMENT, BAZEL_OPTIMIZED_ARGUMENT,
    BAZEL_OUTPUT_ROOT_PREFIX, BAZEL_SYMLINK_PREFIX, E2E_GATE, PACKAGE_TARGET,
};

#[path = "requirements_serial_jobs_tests.rs"]
mod serial_jobs;

const GATE_TARGET_SEPARATOR: &str = " + ";

#[test]
fn package_and_every_gate_are_batch_hermetic_and_bounded() {
    let execution = execution();
    let commands = std::iter::once(PACKAGE_COMMAND)
        .chain(REQUIRED_GATES.into_iter().flat_map(|requirement| {
            requirement
                .bazel_commands()
                .unwrap_or_default()
                .iter()
                .copied()
        }))
        .map(|command| command.arguments(&execution))
        .collect::<Vec<_>>();
    for command in &commands {
        assert_eq!(command[0], BAZEL_BATCH_ARGUMENT);
        assert!(
            command
                .iter()
                .any(|value| value == BAZEL_NO_SYSTEM_RC_ARGUMENT)
        );
        assert!(
            command
                .iter()
                .any(|value| value == BAZEL_NO_HOME_RC_ARGUMENT)
        );
        assert!(
            command
                .iter()
                .any(|value| value == BAZEL_NO_WORKSPACE_RC_ARGUMENT)
        );
        assert!(
            command
                .iter()
                .any(|value| value.starts_with(BAZEL_OUTPUT_ROOT_PREFIX))
        );
        assert!(
            command
                .iter()
                .any(|value| value.starts_with(BAZEL_SYMLINK_PREFIX))
        );
    }
    assert!(
        PACKAGE_COMMAND
            .arguments(&execution)
            .iter()
            .any(|value| value == PACKAGE_TARGET)
    );
    assert!(
        PACKAGE_COMMAND
            .arguments(&execution)
            .iter()
            .any(|value| value == BAZEL_OPTIMIZED_ARGUMENT)
    );
}

#[test]
fn e2e_gate_uses_the_optimized_macos_candidate_configuration() {
    let requirement = REQUIRED_GATES
        .into_iter()
        .find(|requirement| requirement.name == E2E_GATE)
        .unwrap();
    let [gate] = requirement.bazel_commands().unwrap() else {
        panic!("E2E must remain one fixed Bazel invocation");
    };
    let command = gate.arguments(&execution());
    assert!(
        command
            .iter()
            .any(|value| value == BAZEL_MACOS_CONFIG_ARGUMENT)
    );
    assert!(
        command
            .iter()
            .any(|value| value == BAZEL_OPTIMIZED_ARGUMENT)
    );
}

#[test]
fn command_evidence_uses_private_execution_path() {
    let launcher = launcher();
    let execution = execution();
    for (requirement, evidence) in REQUIRED_GATES
        .into_iter()
        .zip(gates(crate::unit_tests::support::REVISION))
    {
        for command in requirement
            .command_evidence(&evidence.tool, &execution)
            .unwrap()
        {
            match &evidence.tool {
                yolo_release_evidence_model::GateToolEvidence::MacOsBazel { .. } => {
                    assert_eq!(command[0], launcher.execution_path);
                    assert_ne!(command[0], launcher.canonical_path);
                }
                yolo_release_evidence_model::GateToolEvidence::LinuxDocker {
                    docker_path,
                    docker_canonical_path,
                    docker_execution_path,
                    ..
                } => {
                    assert_eq!(&command[0], docker_execution_path);
                    assert_ne!(&command[0], docker_path);
                    assert_ne!(&command[0], docker_canonical_path);
                }
            }
        }
    }
}

#[test]
fn linux_gate_target_describes_the_complete_command() {
    assert_eq!(
        yolo_release_contracts::LINUX_GATE_TARGET,
        [
            yolo_release_contracts::LINUX_BUILD_TARGET,
            GATE_TARGET_SEPARATOR,
            yolo_release_contracts::LINUX_TEST_TARGET,
            GATE_TARGET_SEPARATOR,
            yolo_release_contracts::LINUX_CGROUP_TEST_TARGET,
        ]
        .concat()
    );
}

#[test]
fn rust_coverage_gate_consumes_sealed_custody_path() {
    let execution = execution();
    let requirement = REQUIRED_GATES
        .into_iter()
        .find(|requirement| requirement.name == yolo_release_contracts::RUST_COVERAGE_GATE)
        .unwrap();
    let tool = yolo_release_evidence_model::GateToolEvidence::MacOsBazel {
        launcher: launcher(),
    };
    let commands = requirement.command_evidence(&tool, &execution).unwrap();
    assert_eq!(commands.len(), 2);
    assert_eq!(
        commands[1].last().unwrap(),
        &coverage_report_path(&execution).display().to_string()
    );
    assert!(
        !commands[1]
            .last()
            .unwrap()
            .starts_with(&execution.symlink_prefix)
    );
}
