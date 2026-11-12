use super::super::support::execution;
use crate::requirements::{PACKAGE_COMMAND, REQUIRED_GATES};
use yolo_release_contracts::{BAZEL_JOBS_ARGUMENT_PREFIX, SERIAL_JOBS_ARGUMENT};

const SERIAL_JOB_COUNT: usize = 1;
const NON_SERIAL_JOB_COUNT: usize = 2;

fn expected_serial_jobs_argument() -> String {
    format!("{BAZEL_JOBS_ARGUMENT_PREFIX}{SERIAL_JOB_COUNT}")
}

fn has_exact_serial_jobs(arguments: &[String]) -> bool {
    let expected = expected_serial_jobs_argument();
    let mut jobs = arguments
        .iter()
        .filter(|argument| argument.starts_with(BAZEL_JOBS_ARGUMENT_PREFIX));
    matches!(jobs.next(), Some(argument) if argument == &expected) && jobs.next().is_none()
}

fn formal_bazel_commands() -> Vec<Vec<String>> {
    let execution = execution();
    std::iter::once(PACKAGE_COMMAND)
        .chain(REQUIRED_GATES.into_iter().flat_map(|requirement| {
            requirement
                .bazel_commands()
                .unwrap_or_default()
                .iter()
                .copied()
        }))
        .map(|command| command.arguments(&execution))
        .collect()
}

fn non_serial_jobs_argument() -> String {
    format!("{BAZEL_JOBS_ARGUMENT_PREFIX}{NON_SERIAL_JOB_COUNT}")
}

#[test]
fn serial_jobs_constant_is_strictly_one() {
    let expected = expected_serial_jobs_argument();
    assert_eq!(SERIAL_JOBS_ARGUMENT, expected.as_str());
}

#[test]
fn every_formal_bazel_command_has_exactly_one_serial_jobs_argument() {
    for command in formal_bazel_commands() {
        assert!(has_exact_serial_jobs(&command), "{command:?}");
    }
}

#[test]
fn serial_jobs_contract_rejects_a_missing_jobs_argument() {
    assert!(!has_exact_serial_jobs(&Vec::new()));
}

#[test]
fn serial_jobs_contract_rejects_duplicate_jobs_arguments() {
    let command = vec![
        SERIAL_JOBS_ARGUMENT.to_owned(),
        SERIAL_JOBS_ARGUMENT.to_owned(),
    ];
    assert!(!has_exact_serial_jobs(&command));
}

#[test]
fn serial_jobs_contract_rejects_a_trailing_non_serial_override() {
    let command = vec![SERIAL_JOBS_ARGUMENT.to_owned(), non_serial_jobs_argument()];
    assert!(!has_exact_serial_jobs(&command));
}
