use super::super::macos_scan::process_test_support::ReadyProcess;
use super::*;
use crate::process::constants::PTY_SESSION_CAPABILITY_ENV;
use std::process::Stdio;

const REAL_PROCESS_CAPABILITY: &str = "real-process-environment-capability";
const PROTECTED_PROCESS_CAPABILITY: &str = "protected-process-environment-capability";
const PROTECTED_PROCESS_EXECUTABLE: &str = "/bin/sleep";
const PROTECTED_PROCESS_DURATION: &str = "30";
const PROTECTED_PROCESS_ARGUMENT_COUNT: i32 = 2;
const SINGLE_REAL_PROCESS: usize = 1;

#[test]
fn kern_procargs2_detects_a_real_process_environment_capability() {
    let child = ReadyProcess::spawn(REAL_PROCESS_CAPABILITY, Stdio::null());
    let mut budget = CapabilityScanBudget::start(SINGLE_REAL_PROCESS).unwrap();

    let observed = environment_capability(
        child.id(),
        PTY_SESSION_CAPABILITY_ENV,
        REAL_PROCESS_CAPABILITY,
        &mut budget,
    )
    .unwrap();

    assert!(observed.matches);
    assert!(budget.environment_bytes() > usize::default());
}

#[test]
fn argv_only_procargs_payload_is_not_a_false_non_match() {
    let observed = capability_from_arguments(
        &protected_process_argv_only_payload(),
        PTY_SESSION_CAPABILITY_ENV,
        PROTECTED_PROCESS_CAPABILITY,
    );

    assert!(matches!(
        observed,
        Err(CapabilityProofError::CandidateLocalQuery(_))
    ));
}

fn protected_process_argv_only_payload() -> Vec<u8> {
    let mut payload = PROTECTED_PROCESS_ARGUMENT_COUNT.to_ne_bytes().to_vec();
    for field in [
        PROTECTED_PROCESS_EXECUTABLE,
        "",
        PROTECTED_PROCESS_EXECUTABLE,
        PROTECTED_PROCESS_DURATION,
    ] {
        payload.extend_from_slice(field.as_bytes());
        payload.push(FIELD_TERMINATOR);
    }
    payload
}
