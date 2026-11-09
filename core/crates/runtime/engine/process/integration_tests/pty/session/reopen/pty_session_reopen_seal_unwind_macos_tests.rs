use super::*;
use crate::process::pty_session_proof::macos_reopen_seal_panic::{TestSealOperation, panic_next};
use portable_pty::{PtySize, native_pty_system};
use std::panic::{AssertUnwindSafe, catch_unwind};

const TEST_CAPABILITY: &str = "pty-reopen-seal-unwind-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const PANIC_NOT_OBSERVED_DETAIL: &str = "injected reopen-seal panic was not observed";
const SEAL_CUSTODY_LOST_DETAIL: &str = "reopen-seal exact custody was lost during unwind";
const SEAL_PHASE_LOST_DETAIL: &str = "reopen-seal phase changed during unwind";

#[test]
fn activate_panic_retains_prepared_seal_for_retry() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    panic_next(TestSealOperation::Activate);

    assert_panics(|| {
        let _ = proof.acquire_reopen_seal();
    });

    assert_phase(&proof, SealPhase::Prepared);
    let identity = seal_identity(&proof);
    let authority = proof.acquire_reopen_seal().unwrap();
    assert_same_seal(&proof, &identity);
    proof.install_reopen_seal(authority).unwrap();
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
}

#[test]
fn revalidation_panic_retains_acquired_seal_for_retry() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let authority = proof.acquire_reopen_seal().unwrap();
    let identity = seal_identity(&proof);
    panic_next(TestSealOperation::Revalidate);

    assert_panics(|| {
        let _ = proof.install_reopen_seal(authority);
    });

    assert_phase(&proof, SealPhase::Acquired);
    assert_same_seal(&proof, &identity);
    let replacement = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(replacement).unwrap();
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
}

#[test]
fn commit_panic_retains_acquired_seal_for_retry() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    proof.confirm_no_local_master_aliases().unwrap();
    install_seal(&mut proof);
    let identity = seal_identity(&proof);
    panic_next(TestSealOperation::Commit);

    assert_panics(|| {
        let _ = proof.commit_reopen_seal_for_destruction();
    });

    assert_phase(&proof, SealPhase::Acquired);
    assert_same_seal(&proof, &identity);
    proof.commit_reopen_seal_for_destruction().unwrap();
    proof.complete_reopen_seal_without_master_aliases().unwrap();
}

#[test]
fn alias_prepare_panic_retains_committed_seal_for_retry() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    proof
        .bind_runtime(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
        .unwrap();
    let root = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let _release = proof.bind_master_alias_release(root).unwrap();
    install_seal(&mut proof);
    proof.commit_reopen_seal_for_destruction().unwrap();
    let identity = seal_identity(&proof);
    panic_next(TestSealOperation::PrepareMasterAliasClose);

    assert_panics(|| {
        let _ = proof.seal_reopen_before_master_alias_close();
    });

    assert_phase(&proof, SealPhase::Committed);
    assert_same_seal(&proof, &identity);
    proof.seal_reopen_before_master_alias_close().unwrap();
    assert_phase(&proof, SealPhase::AliasesPrepared);
    proof
        .complete_reopen_seal_after_master_alias_close()
        .unwrap();
}

fn assert_panics(operation: impl FnOnce()) {
    assert!(
        catch_unwind(AssertUnwindSafe(operation)).is_err(),
        "{PANIC_NOT_OBSERVED_DETAIL}"
    );
}

fn assert_phase(proof: &PtySessionProof, expected: SealPhase) {
    assert!(
        proof
            .reopen_seal
            .active()
            .is_some_and(|seal| seal.phase == expected),
        "{SEAL_PHASE_LOST_DETAIL}"
    );
}

fn seal_identity(proof: &PtySessionProof) -> MacosPtyReopenSealIdentity {
    proof.reopen_seal.active().unwrap().identity()
}

fn assert_same_seal(proof: &PtySessionProof, identity: &MacosPtyReopenSealIdentity) {
    assert!(
        proof
            .reopen_seal
            .active()
            .is_some_and(|seal| seal.has_identity(identity)),
        "{SEAL_CUSTODY_LOST_DETAIL}"
    );
}

fn install_seal(proof: &mut PtySessionProof) {
    let authority = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(authority).unwrap();
}

fn bound_proof(pair: &portable_pty::PtyPair) -> PtySessionProof {
    PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap()
}

fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}
