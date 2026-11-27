use super::super::macos_scan::process_test_support::ReadyProcess;
use super::*;
use std::path::PathBuf;
use std::process::Stdio;

const RELEASED_MATCH_TTY: &str = "/dev/null";
const RELEASED_MATCH_CAPABILITY: &str = "candidate-released-match";

#[test]
fn released_candidate_matches_environment_before_its_tty_descriptor() {
    let child = ReadyProcess::spawn(RELEASED_MATCH_CAPABILITY, Stdio::null());
    let proof = PtySessionProof::capture(
        PathBuf::from(RELEASED_MATCH_TTY),
        RELEASED_MATCH_CAPABILITY.to_owned(),
    )
    .unwrap();
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();

    assert!(
        proof
            .matches_released_process(child.id(), &mut budget)
            .unwrap()
    );
    let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE).unwrap();
    assert!(
        proof
            .matches_available_process(child.id(), &mut budget)
            .unwrap()
    );
}
