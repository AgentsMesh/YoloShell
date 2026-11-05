use super::test_support::{bound_proof, install_seal, mode, open_pty};
use super::*;
use crate::process::MacosPassivePtyMaster;
use std::panic::{AssertUnwindSafe, catch_unwind};

const MISMATCH_ACCEPTED_DETAIL: &str = "mismatched PTY object acquired a reopen seal";
const FOREIGN_AUTHORITY_ACCEPTED_DETAIL: &str = "foreign reopen-seal authority was accepted";
const SEALED_PROOF_CLONED_DETAIL: &str = "sealed proof clone was accepted";
const SEAL_CUSTODY_LOST_DETAIL: &str = "reopen-seal custody was lost after rejected operation";
const FOREIGN_COMMITTED_ACCEPTED_DETAIL: &str =
    "foreign committed reopen-seal authority was accepted";
const TAMPERED_COMMITTED_ACCEPTED_DETAIL: &str =
    "tampered committed reopen seal passed exact revalidation";
const TAMPERED_COMMITTED_REPAIR_IGNORED_DETAIL: &str =
    "tampered committed reopen seal repair did not invalidate output";
const TAMPERED_COMMITTED_REOPENED_DETAIL: &str =
    "repaired committed reopen seal accepted a new slave open";
const UNCOMMITTED_ACCEPTED_DETAIL: &str = "acquired reopen seal passed committed revalidation";
const TAMPERED_MODE: u32 = (libc::S_IRUSR | libc::S_IWUSR) as u32;

#[test]
fn committed_drop_keeps_the_slave_sealed() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    install_seal(&mut proof);
    proof.commit_reopen_seal_for_destruction().unwrap();

    drop(proof);

    assert_eq!(mode(&path), SEALED_MODE as u32);
}

#[test]
fn mismatched_proof_path_cannot_acquire_a_seal() {
    let pair = open_pty();
    let other = open_pty();
    let mut proof = bound_proof(&pair);
    proof.tty_path = other.master.tty_name().unwrap();

    assert!(
        proof.acquire_reopen_seal().is_err(),
        "{MISMATCH_ACCEPTED_DETAIL}"
    );
}

#[test]
fn foreign_reopen_seal_authority_is_rejected() {
    let owner_pair = open_pty();
    let foreign_pair = open_pty();
    let mut owner = bound_proof(&owner_pair);
    let mut foreign = bound_proof(&foreign_pair);
    let owner_authority = owner.acquire_reopen_seal().unwrap();
    let foreign_authority = foreign.acquire_reopen_seal().unwrap();

    assert!(
        foreign.install_reopen_seal(owner_authority).is_err(),
        "{FOREIGN_AUTHORITY_ACCEPTED_DETAIL}"
    );
    assert!(
        foreign.reopen_seal.active().is_some(),
        "{SEAL_CUSTODY_LOST_DETAIL}"
    );
    foreign.install_reopen_seal(foreign_authority).unwrap();
    let owner_authority = owner.acquire_reopen_seal().unwrap();
    owner.install_reopen_seal(owner_authority).unwrap();
    owner.rollback_reopen_seal_before_commit_for_test().unwrap();
    foreign
        .rollback_reopen_seal_before_commit_for_test()
        .unwrap();
}

#[test]
fn committed_revalidation_rejects_foreign_authority() {
    let owner_pair = open_pty();
    let foreign_pair = open_pty();
    let mut owner = bound_proof(&owner_pair);
    let mut foreign = bound_proof(&foreign_pair);
    let owner_authority = owner.acquire_reopen_seal().unwrap();
    let owner_committed = owner_authority.committed_identity();
    owner.install_reopen_seal(owner_authority).unwrap();
    owner.commit_reopen_seal_for_destruction().unwrap();
    let foreign_authority = foreign.acquire_reopen_seal().unwrap();
    foreign.install_reopen_seal(foreign_authority).unwrap();
    foreign.commit_reopen_seal_for_destruction().unwrap();

    assert!(
        foreign
            .revalidate_committed_reopen_seal(&owner_committed)
            .is_err(),
        "{FOREIGN_COMMITTED_ACCEPTED_DETAIL}"
    );
}

#[test]
fn committed_revalidation_rejects_acquired_seal() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let authority = proof.acquire_reopen_seal().unwrap();
    let committed = authority.committed_identity();
    proof.install_reopen_seal(authority).unwrap();

    assert!(
        proof.revalidate_committed_reopen_seal(&committed).is_err(),
        "{UNCOMMITTED_ACCEPTED_DETAIL}"
    );
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
}

#[test]
fn committed_revalidation_repairs_tampered_current_proof() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let authority = proof.acquire_reopen_seal().unwrap();
    let committed = authority.committed_identity();
    proof.install_reopen_seal(authority).unwrap();
    proof.commit_reopen_seal_for_destruction().unwrap();
    test_support::set_mode(&path, TAMPERED_MODE);

    assert!(
        proof.revalidate_committed_reopen_seal(&committed).is_err(),
        "{TAMPERED_COMMITTED_ACCEPTED_DETAIL}"
    );
    assert!(
        proof.settle_reopen_seal_after_fence().is_err(),
        "{TAMPERED_COMMITTED_REPAIR_IGNORED_DETAIL}"
    );
    assert_eq!(mode(&path), SEALED_MODE as u32);
    assert!(
        io::open_slave(&path).is_err(),
        "{TAMPERED_COMMITTED_REOPENED_DETAIL}"
    );
    proof.settle_reopen_seal_after_fence().unwrap();
}

#[test]
fn sealed_proof_clone_is_rejected_without_losing_custody() {
    let pair = open_pty();
    let mut proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let original_mode = mode(&path);
    install_seal(&mut proof);

    assert!(
        catch_unwind(AssertUnwindSafe(|| proof.clone())).is_err(),
        "{SEALED_PROOF_CLONED_DETAIL}"
    );
    assert!(
        proof.reopen_seal.active().is_some(),
        "{SEAL_CUSTODY_LOST_DETAIL}"
    );
    proof.rollback_reopen_seal_before_commit_for_test().unwrap();
    assert_eq!(mode(&path), original_mode);
}

#[test]
fn finalized_passive_seal_preserves_its_committed_authority() {
    let pair = open_pty();
    let proof = bound_proof(&pair);
    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
    let object = passive.object_identity();
    let mut seal = MacosPtyReopenSeal::prepare_passive(&passive).unwrap();
    seal.activate().unwrap();
    seal.commit_passive(&passive).unwrap();
    let committed = seal.committed_passive(&passive).unwrap();

    let finalized = seal.finalize_passive(&passive).unwrap();
    let retained = seal.committed_passive(&passive).unwrap();

    assert!(committed.validates(&retained, object));
    assert!(finalized.validates(&retained, object));
}

#[test]
fn passive_seal_adopts_an_exact_presealed_slave() {
    let pair = open_pty();
    let proof = bound_proof(&pair);
    let path = proof.tty_path.clone();
    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
    let object = passive.object_identity();
    test_support::set_mode(&path, SEALED_MODE as u32);

    let mut seal = MacosPtyReopenSeal::prepare_passive(&passive).unwrap();
    seal.activate().unwrap();
    seal.commit_passive(&passive).unwrap();
    let committed = seal.committed_passive(&passive).unwrap();
    let finalized = seal.finalize_passive(&passive).unwrap();

    assert!(finalized.validates(&committed, object));
    assert_eq!(mode(&path), SEALED_MODE as u32);
}
