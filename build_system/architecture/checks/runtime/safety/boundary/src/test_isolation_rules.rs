use crate::constants::{
    RUST_HOST_TEST_TARGETS, RUST_REMOTE_TEST_TARGETS, RUST_RESILIENCE_TEST_TARGET,
    TEST_ISOLATION_FORBIDDEN_MARKERS,
};
use crate::test_isolation_contract as contract;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_syntax::targeted_tokens::{Token, tokenize};
#[path = "socket_participant_contract.rs"]
mod participant;

const ISOLATED_CALL: &str = "isolated_rust_test(";
const DIRECT_CALL: &str = "rust_test(";
const REQUIRED_BUILD_TARGETS: &[&str] = &[RUST_RESILIENCE_TEST_TARGET];
const TARGET_MESSAGE: &str = "socket-owning Rust tests must use the isolated test rule";
const RULE_MESSAGE: &str = "isolated Rust tests must preserve coverage and fail-closed execution";
const RUNNER_MESSAGE: &str = "isolated Rust test runner must delegate cleanup to its capability";
const CAPABILITY_MESSAGE: &str = "isolated socket roots must be identity-bound and fail closed";
const LIFECYCLE_MESSAGE: &str =
    "isolated root cleanup must retain exclusive directory inode authority and fair admission";
const COLLECTOR_MESSAGE: &str = "Rust coverage must select the instrumented inner test binary";
const PARTICIPANT_MESSAGE: &str =
    "certified socket cleanup requires current-format directory participant custody";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    for (path, markers) in participant::CONTRACTS {
        if file.relative == *path {
            inspect_code_markers(file, markers, PARTICIPANT_MESSAGE, violations);
        }
    }
    if file.relative == participant::BOOKKEEPING
        && contains_code(&code_tokens(&file.text), participant::REUSED_LEASE_DEADLINE)
    {
        violations.push(Violation::file(&file.relative, PARTICIPANT_MESSAGE));
    }
    match file.relative.as_str() {
        contract::RUNTIME_TESTS => inspect_targets(file, REQUIRED_BUILD_TARGETS, violations),
        contract::HOST_TESTS => inspect_targets(file, RUST_HOST_TEST_TARGETS, violations),
        contract::REMOTE_TESTS => inspect_targets(file, RUST_REMOTE_TEST_TARGETS, violations),
        contract::ISOLATION_RULE => {
            inspect_markers(file, contract::RULE_MARKERS, RULE_MESSAGE, violations)
        }
        contract::COVERAGE_ENVIRONMENT => inspect_markers(
            file,
            contract::COLLECTOR_MARKERS,
            COLLECTOR_MESSAGE,
            violations,
        ),
        contract::ISOLATION_ENTRYPOINT => {
            inspect_runner(file, contract::ENTRYPOINT_MARKERS, violations)
        }
        contract::ISOLATION_RUNNER => inspect_runner(file, contract::RUNNER_MARKERS, violations),
        contract::ISOLATION_WATCHDOG => {
            inspect_runner(file, contract::WATCHDOG_MARKERS, violations)
        }
        contract::ISOLATION_LAUNCHER => {
            inspect_runner(file, contract::LAUNCHER_MARKERS, violations)
        }
        contract::ISOLATION_CUSTODY => {
            inspect_runner(file, contract::CUSTODY_MARKERS, violations);
            if contains_code(&code_tokens(&file.text), contract::REPEATED_ROOT_DROP) {
                violations.push(Violation::file(&file.relative, CAPABILITY_MESSAGE));
            }
        }
        contract::ISOLATION_ROOT => {
            inspect_code_markers(file, contract::ROOT_MARKERS, CAPABILITY_MESSAGE, violations);
        }
        contract::ISOLATION_CREATE => {
            inspect_markers(
                file,
                contract::CREATE_MARKERS,
                CAPABILITY_MESSAGE,
                violations,
            );
            inspect_forbidden(file, CAPABILITY_MESSAGE, violations);
        }
        contract::ISOLATION_FINALIZE => {
            inspect_code_markers(
                file,
                contract::FINALIZE_MARKERS,
                CAPABILITY_MESSAGE,
                violations,
            );
            inspect_forbidden(file, CAPABILITY_MESSAGE, violations);
        }
        contract::LIFECYCLE_LOCK => {
            inspect_code_markers(
                file,
                contract::LIFECYCLE_MARKERS,
                LIFECYCLE_MESSAGE,
                violations,
            );
            inspect_forbidden(file, LIFECYCLE_MESSAGE, violations);
        }
        _ => {}
    }
}

#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/socket_durable_participant_tests.rs"]
mod durable_participant_tests;
#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/socket_participant_contract_tests.rs"]
mod participant_tests;
#[cfg(all(test, feature = "yolo-runtime-boundary-tests"))]
#[path = "../unit_tests/isolated_root_owner_tests.rs"]
mod root_owner_tests;

fn inspect_runner(file: &SourceFile, markers: &[&str], violations: &mut Vec<Violation>) {
    inspect_markers(file, markers, RUNNER_MESSAGE, violations);
    inspect_forbidden(file, RUNNER_MESSAGE, violations);
}

fn inspect_forbidden(file: &SourceFile, message: &str, violations: &mut Vec<Violation>) {
    if TEST_ISOLATION_FORBIDDEN_MARKERS
        .iter()
        .any(|marker| file.text.contains(marker))
    {
        violations.push(Violation::file(&file.relative, message));
    }
}

fn inspect_targets(file: &SourceFile, names: &[&str], violations: &mut Vec<Violation>) {
    for name in names {
        if target_rule(&file.text, name) != Some(ISOLATED_CALL) {
            violations.push(Violation::file(
                &file.relative,
                format!("{TARGET_MESSAGE}: {name}"),
            ));
        }
    }
}

fn target_rule<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let target = format!("name = \"{name}\"");
    let prefix = text.split_once(&target)?.0;
    prefix.lines().rev().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with(ISOLATED_CALL) {
            Some(ISOLATED_CALL)
        } else if trimmed.starts_with(DIRECT_CALL) {
            Some(DIRECT_CALL)
        } else {
            None
        }
    })
}

fn inspect_markers(
    file: &SourceFile,
    markers: &[&str],
    message: &str,
    violations: &mut Vec<Violation>,
) {
    if markers.iter().any(|marker| !file.text.contains(marker)) {
        violations.push(Violation::file(&file.relative, message));
    }
}

fn inspect_code_markers(
    file: &SourceFile,
    markers: &[&str],
    message: &str,
    violations: &mut Vec<Violation>,
) {
    let tokens = code_tokens(&file.text);
    if markers.iter().any(|marker| !contains_code(&tokens, marker)) {
        violations.push(Violation::file(&file.relative, message));
    }
}

fn contains_code(tokens: &[Token], marker: &str) -> bool {
    let expected = code_tokens(marker);
    tokens.windows(expected.len()).any(|candidate| {
        candidate
            .iter()
            .zip(&expected)
            .all(|(left, right)| left.kind == right.kind)
    })
}

fn code_tokens(text: &str) -> Vec<Token> {
    tokenize(text)
        .into_iter()
        .filter(|token| !token.symbol(","))
        .collect()
}
