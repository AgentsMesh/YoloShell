use super::test_support::{bound_proof, install_seal, mode, open_pty, set_mode};
use super::*;
use crate::process::pty_session_proof::macos_reopen_seal_io::{
    TestIoOperation, fail_next, open_slave,
};

const REOPEN_ALLOWED_DETAIL: &str = "sealed PTY slave accepted a new open";
const MODE_NOT_RESTORED_DETAIL: &str = "PTY slave mode was not restored";
const FAILURE_IGNORED_DETAIL: &str = "injected reopen-seal failure was ignored";
const REPLACEMENT_CHANGED_DETAIL: &str = "replacement PTY object was chmod during rollback";
const TAMPER_ACCEPTED_DETAIL: &str = "tampered reopen seal was accepted during install";
const CONSECUTIVE_RESTORE_FAILURES: usize = 2;

#[test]
fn precommit_rollback_restores_mode_and_reopen_access() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    install_seal(&mut proof);
    assert!(open_slave(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");

    proof.rollback_reopen_seal_before_commit_for_test().unwrap();

    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
    drop(open_slave(&path).unwrap());
}

#[test]
fn precommit_drop_restores_mode_and_reopen_access() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    install_seal(&mut proof);

    drop(proof);

    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
    drop(open_slave(&path).unwrap());
}

#[test]
fn consecutive_restore_failures_remain_non_reopenable() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    install_seal(&mut proof);

    for _attempt in 0..CONSECUTIVE_RESTORE_FAILURES {
        fail_next(TestIoOperation::ChangeMode);
        assert!(
            proof.rollback_reopen_seal_before_commit_for_test().is_err(),
            "{FAILURE_IGNORED_DETAIL}"
        );
        assert_eq!(mode(&path), SEALED_MODE as u32);
        assert!(open_slave(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");
    }
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
}

#[test]
fn rollback_uses_retained_descriptor_without_reopening() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    install_seal(&mut proof);
    fail_next(TestIoOperation::OpenSlave);

    proof.rollback_reopen_seal_before_commit_for_test().unwrap();

    assert_eq!(mode(&path), original, "{MODE_NOT_RESTORED_DETAIL}");
    assert!(open_slave(&path).is_err(), "{FAILURE_IGNORED_DETAIL}");
    drop(open_slave(&path).unwrap());
}

#[test]
fn install_revalidates_externally_tampered_mode() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original = mode(&path);
    let authority = proof.acquire_reopen_seal().unwrap();
    set_mode(&path, original);

    assert!(
        proof.install_reopen_seal(authority).is_err(),
        "{TAMPER_ACCEPTED_DETAIL}"
    );
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
}

#[test]
fn rollback_uses_exact_descriptor_when_validation_path_changes() {
    let pair = open_pty();
    let replacement = open_pty();
    let mut proof = bound_proof(&pair);
    let original_path = proof.tty_path.clone();
    let original_mode = mode(&original_path);
    let replacement_path = replacement.master.tty_name().unwrap();
    let replacement_mode = mode(&replacement_path);
    install_seal(&mut proof);
    proof
        .reopen_seal
        .active_mut()
        .unwrap()
        .replace_validation_path_for_test(replacement_path.clone());

    proof.rollback_reopen_seal_before_commit_for_test().unwrap();

    assert_eq!(
        mode(&replacement_path),
        replacement_mode,
        "{REPLACEMENT_CHANGED_DETAIL}"
    );
    assert_eq!(
        mode(&original_path),
        original_mode,
        "{MODE_NOT_RESTORED_DETAIL}"
    );
}
