use super::*;
use crate::process::PtySessionProof;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHolderIdentity, CustodyLeaseId, CustodyOffer, CustodyProfile,
    CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use std::path::PathBuf;

const TEST_CAPABILITY: &str = "fallback-release-control";
const TEST_DEVICE: &str = "/dev/null";
const TEST_AUTH_TOKEN: &str = "fallback-release-auth";
const TEST_BOOT_ID: &str = "fallback-release-boot";
const TEST_GENERATION: u64 = 31;
const TEST_ROOT_PID: u32 = 45_101;
const TEST_ROOT_START: u64 = 83_101;
const TEST_WORKER_PID: u32 = 45_102;
const TEST_WORKER_START: u64 = 83_102;
const RECOVERY_DIGEST_BYTE: u8 = 0x71;
const FOREIGN_RECOVERY_DIGEST_BYTE: u8 = 0x72;
const TEST_READER_HANDLE: u64 = 91_101;
const TEST_WRITER_HANDLE: u64 = 91_102;

#[test]
fn release_stays_exact_until_global_exclusion_is_committed() {
    let (proof, authority, runtime) = bound_release();
    let recovery = digest(RECOVERY_DIGEST_BYTE);
    proof
        .bind_secondary_custody_recovery(&runtime, recovery)
        .unwrap();
    let (_, verifier) = authority.take_local_aliases_closed().unwrap();

    assert_exact(&verifier.release_control(&offer(runtime, recovery)).unwrap());
}

#[test]
fn committed_global_exclusion_mints_only_a_bound_macos_fallback() {
    let (proof, authority, runtime) = bound_release();
    let recovery = digest(RECOVERY_DIGEST_BYTE);
    proof
        .bind_secondary_custody_recovery(&runtime, recovery)
        .unwrap();
    proof.commit_confirmed_bootstrap_global_exclusion();
    let (_, verifier) = authority.take_local_aliases_closed().unwrap();
    let exact_offer = offer(runtime.clone(), recovery);

    assert!(matches!(
        verifier
            .release_control(&exact_offer)
            .unwrap()
            .release_settlement(),
        Some(
            crate::pty_custody::CustodyReleaseSettlement::ConfirmedBootstrapGlobalExclusion { .. }
        )
    ));

    let released =
        offer_with_descriptor_set(runtime, recovery, CustodyDescriptorSet::ReleasedSuccessor);
    assert_exact(&verifier.release_control(&released).unwrap());
}

#[test]
fn committed_fallback_rejects_foreign_runtime_and_recovery_identity() {
    let (proof, authority, runtime) = bound_release();
    let recovery = digest(RECOVERY_DIGEST_BYTE);
    proof
        .bind_secondary_custody_recovery(&runtime, recovery)
        .unwrap();
    proof.commit_confirmed_bootstrap_global_exclusion();
    let (_, verifier) = authority.take_local_aliases_closed().unwrap();

    let mut foreign_runtime = runtime.clone();
    foreign_runtime.runtime_epoch = Uuid::new_v4();
    assert!(
        verifier
            .release_control(&offer(foreign_runtime, recovery))
            .is_err()
    );
    assert!(
        verifier
            .release_control(&offer(runtime, digest(FOREIGN_RECOVERY_DIGEST_BYTE)))
            .is_err()
    );
}

fn bound_release() -> (
    PtySessionProof,
    PtyMasterAliasReleaseAuthority,
    CustodyRuntimeIdentity,
) {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let root = ProcessIdentity {
        pid: TEST_ROOT_PID,
        start_time: TEST_ROOT_START,
    };
    let mut proof =
        PtySessionProof::capture(PathBuf::from(TEST_DEVICE), TEST_CAPABILITY.to_owned()).unwrap();
    proof.bind_runtime(terminal_id, runtime_epoch).unwrap();
    let authority = proof.bind_master_alias_release(root).unwrap();
    let runtime = CustodyRuntimeIdentity {
        terminal_id,
        runtime_epoch,
        worker_id: Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START,
        child_pid: root.pid,
        child_start_time: root.start_time,
        boot_id: TEST_BOOT_ID.to_owned(),
    };
    (proof, authority, runtime)
}

fn offer(runtime: CustodyRuntimeIdentity, recovery: Sha256Digest) -> CustodyOffer {
    offer_with_descriptor_set(
        runtime,
        recovery,
        CustodyDescriptorSet::macos_process_capability(TEST_READER_HANDLE, TEST_WRITER_HANDLE),
    )
}

fn offer_with_descriptor_set(
    runtime: CustodyRuntimeIdentity,
    recovery: Sha256Digest,
    descriptor_set: CustodyDescriptorSet,
) -> CustodyOffer {
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime,
        CustodyHolderIdentity::capture().unwrap(),
        TEST_AUTH_TOKEN.to_owned(),
        TEST_GENERATION,
    );
    let profile = match descriptor_set {
        CustodyDescriptorSet::ReleasedSuccessor => CustodyProfile::ReleasedTerminal,
        _ => CustodyProfile::ActiveRunning,
    };
    CustodyOffer::new_with_profile(&request, descriptor_set, profile, recovery).unwrap()
}

fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::repeated(byte)
}

fn assert_exact(control: &crate::pty_custody::CustodyControl) {
    assert!(matches!(
        control.release_settlement(),
        Some(crate::pty_custody::CustodyReleaseSettlement::ExactProcessCapability)
    ));
}
