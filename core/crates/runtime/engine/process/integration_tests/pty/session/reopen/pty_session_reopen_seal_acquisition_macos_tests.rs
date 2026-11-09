use super::test_support::{bound_proof, install_seal, mode, open_pty};
use crate::process::pty_session_proof::macos_reopen_seal_io::{TestIoOperation, fail_next};

const MODE_NOT_RESTORED_DETAIL: &str = "PTY slave mode was not restored";
const FAILURE_IGNORED_DETAIL: &str = "injected reopen-seal failure was ignored";

#[test]
fn validation_failure_retains_seal_for_acquisition_retry() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    fail_next(TestIoOperation::ValidateSeal);

    assert!(
        proof.acquire_reopen_seal().is_err(),
        "{FAILURE_IGNORED_DETAIL}"
    );
    install_seal(&mut proof);
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
}

#[test]
fn slave_open_failure_leaves_mode_unchanged() {
    assert_acquisition_failure_rolls_back(TestIoOperation::OpenSlave);
}

#[test]
fn slave_fchmod_failure_leaves_mode_unchanged() {
    assert_acquisition_failure_rolls_back(TestIoOperation::ChangeMode);
}

fn assert_acquisition_failure_rolls_back(operation: TestIoOperation) {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    fail_next(operation);
    assert!(
        proof.acquire_reopen_seal().is_err(),
        "{FAILURE_IGNORED_DETAIL}"
    );
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
}
