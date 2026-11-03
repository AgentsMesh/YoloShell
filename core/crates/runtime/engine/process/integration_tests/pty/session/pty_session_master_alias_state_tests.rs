use super::*;
use crate::process::ProcessIdentity;
use std::path::PathBuf;
use uuid::Uuid;

const TEST_TTY_PATH: &str = "/dev/null";
const TEST_CAPABILITY: &str = "typed-master-alias-state";
const TEST_PID: u32 = 51_301;
const TEST_START_TIME: u64 = 91_301;
#[cfg(target_os = "macos")]
const TEST_WORKER_PID: u32 = 51_302;
#[cfg(target_os = "macos")]
const TEST_WORKER_START_TIME: u64 = 91_302;
#[cfg(target_os = "macos")]
const TEST_BOOT_ID: &str = "typed-master-alias-boot";
#[cfg(target_os = "macos")]
const TEST_DIGEST_BYTE: u8 = 0x61;
const STATE_CHANGED_DETAIL: &str = "rejected transition changed the master-alias state";

#[test]
fn alias_bound_transition_is_one_shot_and_retains_its_exact_verifier() {
    let mut proof = pty_proof();
    assert!(matches!(
        &proof.master_alias_state,
        PtyMasterAliasState::Unbound
    ));
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let release = proof.bind_master_alias_release(root()).unwrap();
    assert!(matches!(
        &proof.master_alias_state,
        PtyMasterAliasState::AliasBound(_)
    ));

    assert!(proof.confirm_no_local_master_aliases().is_err());
    assert!(proof.bind_master_alias_release(root()).is_err());
    assert!(
        matches!(
            &proof.master_alias_state,
            PtyMasterAliasState::AliasBound(_)
        ),
        "{STATE_CHANGED_DETAIL}"
    );
    assert!(proof.process_closure_identity().is_some());
    assert!(proof.validates_master_aliases_closed(&release.prove_closed_for_test()));
    #[cfg(target_os = "macos")]
    assert!(proof.master_alias_release_requirement().unwrap());
}

#[test]
fn no_local_alias_transition_is_one_shot_and_cannot_mint_a_verifier() {
    let mut proof = pty_proof();
    proof.confirm_no_local_master_aliases().unwrap();
    assert_no_local_alias(&proof);

    assert!(proof.confirm_no_local_master_aliases().is_err());
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    assert!(proof.bind_master_alias_release(root()).is_err());
    assert_no_local_alias(&proof);
    assert!(proof.process_closure_identity().is_none());
    assert!(std::panic::catch_unwind(|| proof.clone()).is_err());
    #[cfg(target_os = "macos")]
    assert!(!proof.master_alias_release_requirement().unwrap());
}

#[test]
fn unbound_state_cannot_validate_foreign_alias_closure() {
    let proof = pty_proof();
    let closed = bound_closed();

    assert!(!proof.validates_master_aliases_closed(&closed));
    assert!(proof.process_closure_identity().is_none());
    #[cfg(target_os = "macos")]
    assert!(proof.master_alias_release_requirement().is_err());
}

#[test]
fn discovery_no_alias_resolution_survives_a_safe_clone() {
    let proof = PtySessionProof::discovery(TEST_CAPABILITY.to_owned()).unwrap();
    let clone = proof.clone();

    assert_no_local_alias(&proof);
    assert_no_local_alias(&clone);
    #[cfg(target_os = "macos")]
    assert!(!proof.master_alias_release_requirement().unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn alias_dependent_custody_rejects_unbound_and_no_local_states() {
    let runtime = custody_runtime();
    let digest = crate::sha256_digest::Sha256Digest::repeated(TEST_DIGEST_BYTE);
    let mut no_alias = pty_proof();
    no_alias.confirm_no_local_master_aliases().unwrap();

    assert!(
        pty_proof()
            .bind_secondary_custody_recovery(&runtime, digest)
            .is_err()
    );
    assert!(
        no_alias
            .bind_secondary_custody_recovery(&runtime, digest)
            .is_err()
    );
}

fn pty_proof() -> PtySessionProof {
    PtySessionProof::capture(PathBuf::from(TEST_TTY_PATH), TEST_CAPABILITY.to_owned()).unwrap()
}

fn bound_closed() -> crate::process::PtyMasterAliasesClosed {
    let mut proof = pty_proof();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    proof
        .bind_master_alias_release(root())
        .unwrap()
        .prove_closed_for_test()
}

fn root() -> ProcessIdentity {
    ProcessIdentity {
        pid: TEST_PID,
        start_time: TEST_START_TIME,
    }
}

fn assert_no_local_alias(proof: &PtySessionProof) {
    assert!(
        matches!(&proof.master_alias_state, PtyMasterAliasState::NoLocalAlias),
        "{STATE_CHANGED_DETAIL}"
    );
}

#[cfg(target_os = "macos")]
fn custody_runtime() -> crate::pty_custody::CustodyRuntimeIdentity {
    crate::pty_custody::CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START_TIME,
        child_pid: TEST_PID,
        child_start_time: TEST_START_TIME,
        boot_id: TEST_BOOT_ID.to_owned(),
    }
}
