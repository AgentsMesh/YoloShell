use super::test_support::{bound_proof, mode, open_pty, set_mode};
use super::*;
use crate::process::pty_session_proof::macos_reopen_seal_io::{
    TestIoOperation, fail_next, open_slave,
};

const WRITE_ONLY_MODE: u32 = libc::S_IWUSR as u32;
const PARTIAL_TAMPER_MODES: &[u32] = &[
    libc::S_IRUSR as u32,
    libc::S_IWUSR as u32,
    libc::S_IXUSR as u32,
    libc::S_IRGRP as u32,
    libc::S_IWGRP as u32,
    libc::S_IXGRP as u32,
    libc::S_IROTH as u32,
    libc::S_IWOTH as u32,
    libc::S_IXOTH as u32,
];
const REPAIR_ACCEPTED_DETAIL: &str = "tampered committed seal did not invalidate output";
const REPAIR_FAILED_DETAIL: &str = "partial PTY mode was not repaired to the exact seal";
const REOPEN_ALLOWED_DETAIL: &str = "repaired PTY seal accepted a new slave open";
const QUARANTINE_FAILURE_IGNORED_DETAIL: &str = "injected quarantine failure was ignored";
const DROP_RESTORED_MODE_DETAIL: &str =
    "failed quarantine Drop restored the original permissive PTY mode";

#[test]
fn committed_repair_seals_every_permission_class_tamper() {
    for tampered_mode in PARTIAL_TAMPER_MODES {
        assert_committed_repair(*tampered_mode);
    }
}

#[test]
fn failed_quarantine_drop_keeps_the_physical_seal() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original_mode = mode(&path);
    acquire_seal(&mut proof);
    proof.mark_reopen_seal_destructive_fence_crossed().unwrap();
    set_mode(&path, WRITE_ONLY_MODE);
    fail_next(TestIoOperation::FinalizeQuarantine);

    assert!(
        proof.quarantine_reopen_seal().is_err(),
        "{QUARANTINE_FAILURE_IGNORED_DETAIL}"
    );
    assert_eq!(mode(&path), SEALED_MODE as u32, "{REPAIR_FAILED_DETAIL}");
    drop(proof);

    assert_eq!(mode(&path), SEALED_MODE as u32, "{REPAIR_FAILED_DETAIL}");
    assert_ne!(mode(&path), original_mode, "{DROP_RESTORED_MODE_DETAIL}");
    assert!(open_slave(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");
}

fn assert_committed_repair(tampered_mode: u32) {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    commit_seal(&mut proof);
    set_mode(&path, tampered_mode);

    assert!(
        proof.settle_reopen_seal_after_fence().is_err(),
        "{REPAIR_ACCEPTED_DETAIL}"
    );
    assert_eq!(
        mode(&path),
        SEALED_MODE as u32,
        "{REPAIR_FAILED_DETAIL}: {tampered_mode:o}"
    );
    assert!(open_slave(&path).is_err(), "{REOPEN_ALLOWED_DETAIL}");
    proof.settle_reopen_seal_after_fence().unwrap();
}

fn commit_seal(proof: &mut PtySessionProof) {
    acquire_seal(proof);
    proof.commit_reopen_seal_for_destruction().unwrap();
}

fn acquire_seal(proof: &mut PtySessionProof) {
    let authority = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(authority).unwrap();
}
