use super::{
    constants::{
        TEST_EXECUTABLE_ENVIRONMENT, TEST_FILTER_ENVIRONMENT, TEST_SOURCE_DIRECTORY_ENVIRONMENT,
        TEST_WORKSPACE_ENVIRONMENT,
    },
    environment::{
        canonical_executable, required_environment, resolve_executable, validate_executable,
        validate_relative,
    },
    launcher::child_command,
};
use crate::main;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use yolo_paths::SOCKET_ROOT_ENVIRONMENT;

const EXECUTABLE: &str = "/private/tmp/yolo runner; false";
const SOCKET_ROOT: &str = "/private/tmp/yolo socket root";
const EXISTING_ARGUMENT: &str = "existing harness argument";
const EXISTING_VALUE: &str = "existing harness value";
const TEST_FILTER: &str = "worker::tests::recovers_backpressure";
const NON_UTF8_BYTE: u8 = 0x80;
const TEST_REQUIRED_ENVIRONMENT: &str = "YOLO_SOCKET_GUARD_REQUIRED_TEST";
const SOURCE_DIRECTORY: &str = "/";
const WORKSPACE_DIRECTORY: &str = "";
const TRUE_EXECUTABLE_RELATIVE: &str = crate::test_executables::TRUE_RELATIVE;
const TRUE_EXECUTABLE: &str = crate::test_executables::TRUE;
const DIRECTORY_INSTEAD_OF_EXECUTABLE: &str = "/private/tmp";
const MISSING_EXECUTABLE: &str = "/private/tmp/yolo-missing-socket-guard-executable";
const PARENT_RELATIVE: &str = "../usr/bin/true";
const ABSOLUTE_RELATIVE: &str = "/usr/bin/true";
const NORMAL_RELATIVE: &str = "nested/test-binary";
const EXECUTABLE_LINK_NAME: &str = "test-binary";
const HOSTILE_FILTERS: &[&str] = &[
    "   ",
    "filter with spaces",
    "filter with\na newline",
    "; touch /private/tmp/yolo-filter-injection",
    "$(touch /private/tmp/yolo-filter-substitution)",
    "`touch /private/tmp/yolo-filter-backticks`",
    "\"double quoted\" 'single quoted' \\ escaped",
];

fn command(arguments: &[&str], test_filter: Option<OsString>) -> std::process::Command {
    child_command(
        Path::new(EXECUTABLE),
        Path::new(SOCKET_ROOT),
        arguments.iter().map(|value| OsString::from(*value)),
        test_filter,
    )
}

fn arguments(command: &std::process::Command) -> Vec<&OsStr> {
    command.get_args().collect()
}

#[test]
fn absent_filter_preserves_existing_arguments() {
    let command = command(&[EXISTING_ARGUMENT, EXISTING_VALUE], None);
    assert_eq!(
        arguments(&command),
        [OsStr::new(EXISTING_ARGUMENT), OsStr::new(EXISTING_VALUE)]
    );
}

#[test]
fn empty_filter_does_not_create_an_empty_harness_argument() {
    let command = command(&[EXISTING_ARGUMENT], Some(OsString::new()));
    assert_eq!(arguments(&command), [OsStr::new(EXISTING_ARGUMENT)]);
}

#[test]
fn filter_is_appended_after_existing_harness_arguments() {
    let command = command(&[EXISTING_ARGUMENT], Some(OsString::from(TEST_FILTER)));
    assert_eq!(
        arguments(&command),
        [OsStr::new(EXISTING_ARGUMENT), OsStr::new(TEST_FILTER)]
    );
}

#[test]
fn adversarial_filter_text_remains_one_literal_argument() {
    for filter in HOSTILE_FILTERS {
        let filter = OsString::from(filter);
        let command = command(&[], Some(filter.clone()));
        assert_eq!(command.get_program(), OsStr::new(EXECUTABLE));
        assert_eq!(arguments(&command), [filter.as_os_str()]);
    }
}

#[test]
fn non_utf8_filter_is_forwarded_without_conversion() {
    let filter = OsString::from_vec(vec![b'f', b'i', b'l', b't', b'e', b'r', NON_UTF8_BYTE]);
    let command = command(&[], Some(filter.clone()));
    assert_eq!(arguments(&command), [filter.as_os_str()]);
}

#[test]
fn consumed_filter_is_removed_from_the_child_environment() {
    let command = command(&[], Some(OsString::from(TEST_FILTER)));
    let environment: Vec<_> = command.get_envs().collect();
    assert!(environment.contains(&(OsStr::new(TEST_FILTER_ENVIRONMENT), None)));
    assert!(environment.contains(&(
        OsStr::new(SOCKET_ROOT_ENVIRONMENT),
        Some(OsStr::new(SOCKET_ROOT)),
    )));
}

#[test]
fn runner_resolves_validates_and_restores_environment() {
    let guard = EnvironmentRestore::capture(&[
        TEST_SOURCE_DIRECTORY_ENVIRONMENT,
        TEST_WORKSPACE_ENVIRONMENT,
        TEST_EXECUTABLE_ENVIRONMENT,
        TEST_REQUIRED_ENVIRONMENT,
    ]);
    guard.remove(TEST_REQUIRED_ENVIRONMENT);
    assert!(required_environment(TEST_REQUIRED_ENVIRONMENT).is_err());
    guard.set(TEST_REQUIRED_ENVIRONMENT, EXISTING_VALUE);
    assert_eq!(
        required_environment(TEST_REQUIRED_ENVIRONMENT).unwrap(),
        OsString::from(EXISTING_VALUE)
    );

    for invalid in ["", PARENT_RELATIVE, ABSOLUTE_RELATIVE] {
        assert!(
            validate_relative(Path::new(invalid)).is_err(),
            "{invalid:?}"
        );
    }
    validate_relative(Path::new(NORMAL_RELATIVE)).unwrap();
    validate_executable(Path::new(TRUE_EXECUTABLE)).unwrap();
    assert!(validate_executable(Path::new(DIRECTORY_INSTEAD_OF_EXECUTABLE)).is_err());
    assert!(validate_executable(Path::new(MISSING_EXECUTABLE)).is_err());

    guard.remove(TEST_EXECUTABLE_ENVIRONMENT);
    assert_eq!(main(), ExitCode::FAILURE);
    guard.set(TEST_SOURCE_DIRECTORY_ENVIRONMENT, SOURCE_DIRECTORY);
    guard.set(TEST_WORKSPACE_ENVIRONMENT, WORKSPACE_DIRECTORY);
    guard.set(TEST_EXECUTABLE_ENVIRONMENT, TRUE_EXECUTABLE_RELATIVE);
    assert_eq!(
        resolve_executable().unwrap(),
        PathBuf::from(TRUE_EXECUTABLE)
    );
}

#[test]
fn runner_executes_the_canonical_binary_behind_a_runfiles_link() {
    let directory = tempfile::tempdir().unwrap();
    let link = directory.path().join(EXECUTABLE_LINK_NAME);
    std::os::unix::fs::symlink(TRUE_EXECUTABLE, &link).unwrap();
    assert_eq!(
        canonical_executable(&link).unwrap(),
        Path::new(TRUE_EXECUTABLE).canonicalize().unwrap()
    );
}

struct EnvironmentRestore {
    values: Vec<(&'static str, Option<OsString>)>,
}

impl EnvironmentRestore {
    fn capture(names: &[&'static str]) -> Self {
        Self {
            values: names
                .iter()
                .map(|name| (*name, std::env::var_os(name)))
                .collect(),
        }
    }

    fn set(&self, name: &'static str, value: &str) {
        unsafe { std::env::set_var(name, value) };
    }

    fn remove(&self, name: &'static str) {
        unsafe { std::env::remove_var(name) };
    }
}

impl Drop for EnvironmentRestore {
    fn drop(&mut self) {
        for (name, value) in &self.values {
            match value {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}
