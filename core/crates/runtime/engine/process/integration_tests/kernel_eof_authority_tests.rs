use super::*;
use crate::process::pty_session_proof::PtyReopenSealAuthority;
use crate::process::{ProcessIdentity, PtySessionProof};
use std::path::PathBuf;

const TEST_DEVICE_PATH: &str = "/dev/null";
const TARGET_CAPABILITY: &str = "kernel-eof-target";
const FOREIGN_CAPABILITY: &str = "kernel-eof-foreign";
const TEST_PID: u32 = 64_101;
const TEST_START_TIME: u64 = 97_101;

#[test]
fn stale_version_cannot_authorize_kernel_eof() {
    let (binding, version) = bound(TARGET_CAPABILITY);
    let output = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_kernel_eof(&version, &binding, &output).unwrap();

    assert!(
        permit
            .authorize(&ProcessClosureVersion::default())
            .is_none()
    );
}

#[test]
fn foreign_master_alias_cannot_bind_target_version() {
    let (_target, version) = bound(TARGET_CAPABILITY);
    let (foreign, _) = bound(FOREIGN_CAPABILITY);
    let output = KernelPtyEofIdentity::from_kernel_read();

    assert!(bind_kernel_eof(&version, &foreign, &output).is_none());
}

#[test]
fn foreign_eof_identity_cannot_satisfy_commit_binding() {
    let (binding, version) = bound(TARGET_CAPABILITY);
    let target = KernelPtyEofIdentity::from_kernel_read();
    let foreign = KernelPtyEofIdentity::from_kernel_read();
    let (permit, validation) = bind_kernel_eof(&version, &binding, &target).unwrap();
    let (_, foreign_validation) = bind_kernel_eof(&version, &binding, &foreign).unwrap();
    let commit = permit.authorize(&version).unwrap();
    let foreign = foreign_validation
        .validate_current(&version, &binding, &foreign)
        .unwrap();
    let seal = PtyReopenSealAuthority::missing_proof().committed_identity();
    let foreign = foreign.bind_reopen_seal_for_test(&seal);

    assert!(!commit.consume_sealed(&seal, foreign));
    let target = validation
        .validate_current(&version, &binding, &target)
        .unwrap();
    let target = target.bind_reopen_seal_for_test(&seal);
    assert!(commit.consume_sealed(&seal, target));
}

fn bound(capability: &str) -> (Arc<PtyMasterAliasReleaseAuthority>, ProcessClosureVersion) {
    let mut proof =
        PtySessionProof::capture(PathBuf::from(TEST_DEVICE_PATH), capability.to_owned()).unwrap();
    proof
        .bind_runtime(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
        .unwrap();
    let binding = Arc::new(
        proof
            .bind_master_alias_release(ProcessIdentity {
                pid: TEST_PID,
                start_time: TEST_START_TIME,
            })
            .unwrap(),
    );
    let version = ProcessClosureVersion::for_pty_output_test(&binding);
    (binding, version)
}
