use super::*;
use crate::process::PtySessionProof;
use crate::pty_custody::ExactCustodyReleasedAck;
use std::path::PathBuf;

const TEST_DEVICE_PATH: &str = "/dev/null";
const TEST_CAPABILITY: &str = "typed-master-release-settlement";
const TEST_PID: u32 = 43_101;
const TEST_START_TIME: u64 = 79_101;

#[test]
fn local_closure_and_exact_host_ack_produce_the_bound_final_proof() {
    let (proof, authority) = bound_release();
    let (local, verifier) = authority.take_local_aliases_closed().unwrap();
    let acknowledgement = verifier.acknowledge(ExactCustodyReleasedAck::for_test());
    let closed = local
        .finalize_host_release(acknowledgement)
        .unwrap_or_else(|(_local, error)| panic!("{error:#}"))
        .mint_final_proof();

    assert!(proof.validates_master_aliases_closed(&closed));
}

#[test]
fn foreign_host_ack_cannot_finalize_another_local_closure() {
    let (_, first) = bound_release();
    let (_, second) = bound_release();
    let (_, first_verifier) = first.take_local_aliases_closed().unwrap();
    let (second_local, _) = second.take_local_aliases_closed().unwrap();
    let foreign = first_verifier.acknowledge(ExactCustodyReleasedAck::for_test());

    assert!(second_local.finalize_host_release(foreign).is_err());
}

#[test]
fn local_closure_authority_is_one_shot() {
    let (_, authority) = bound_release();
    let _ = authority.take_local_aliases_closed().unwrap();
    assert!(authority.take_local_aliases_closed().is_err());
}

#[test]
fn production_finalizers_are_consuming_and_have_no_parameterless_bypass() {
    let settlement = include_str!("../../../src/pty/pty_master_release_settlement.rs");
    let authority = include_str!("../../../src/pty/pty_master_release.rs");
    assert!(settlement.contains("fn mint_final_proof(self)"));
    assert!(!settlement.contains("fn acknowledge(self)"));
    assert!(!authority.contains("fn prove_closed("));
}

fn bound_release() -> (PtySessionProof, PtyMasterAliasReleaseAuthority) {
    let mut proof =
        PtySessionProof::capture(PathBuf::from(TEST_DEVICE_PATH), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let release = proof
        .bind_master_alias_release(ProcessIdentity {
            pid: TEST_PID,
            start_time: TEST_START_TIME,
        })
        .unwrap();
    (proof, release)
}
