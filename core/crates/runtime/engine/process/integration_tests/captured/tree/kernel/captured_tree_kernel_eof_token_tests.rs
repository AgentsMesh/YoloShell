use super::exact_gate_fixture::ExactGateFixture;
use super::kernel_eof_test_support::prepare_after_fresh_eof;
use crate::process::kernel_eof_authority::{KernelPtyEofIdentity, bind_scripted_kernel_eof};
use crate::process::{PtySessionProof, install_test_signal_hook};
use portable_pty::native_pty_system;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const TEST_CAPABILITY: &str = "kernel-eof-token";
const FIRST_SIGNAL_TARGET: usize = 0;
const FOREIGN_TOKEN_ACCEPTED_DETAIL: &str = "foreign kernel EOF token authorized SIGKILL";
const FOREIGN_SEAL_ACCEPTED_DETAIL: &str = "foreign reopen seal authorized SIGKILL";

#[test]
fn foreign_validated_eof_cannot_reach_sigkill() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut frozen = fixture.take_kernel_eof_frozen();
    let version = frozen.closure_version();
    let (prepared, _validated) = prepare_after_fresh_eof(&mut frozen);
    let output = KernelPtyEofIdentity::from_kernel_read();
    let (_, validation) = bind_scripted_kernel_eof(&version, &output);
    let foreign = validation.validate_scripted(&version, &output).unwrap();
    let foreign = frozen.seal_kernel_eof_for_test(&prepared, foreign);
    let signaled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&signaled);
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        observed.store(true, Ordering::Release);
    });

    assert!(
        frozen
            .commit_kill_with_kernel_eof(prepared, foreign)
            .is_err()
    );
    assert!(
        !signaled.load(Ordering::Acquire),
        "{FOREIGN_TOKEN_ACCEPTED_DETAIL}"
    );
}

#[test]
fn foreign_committed_seal_cannot_reach_sigkill() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut frozen = fixture.take_kernel_eof_frozen();
    let (prepared, validated) = prepare_after_fresh_eof(&mut frozen);
    let foreign_pair = native_pty_system()
        .openpty(super::exact_gate_authority_tests::pty_size())
        .unwrap();
    let mut foreign_proof =
        PtySessionProof::capture_bound(foreign_pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    let foreign_authority = foreign_proof.acquire_reopen_seal().unwrap();
    let foreign_seal = foreign_authority.committed_identity();
    foreign_proof
        .install_reopen_seal(foreign_authority)
        .unwrap();
    foreign_proof.commit_reopen_seal_for_destruction().unwrap();
    let foreign = validated.bind_reopen_seal_for_test(&foreign_seal);
    let signaled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&signaled);
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        observed.store(true, Ordering::Release);
    });

    assert!(
        frozen
            .commit_kill_with_kernel_eof(prepared, foreign)
            .is_err()
    );
    assert!(
        !signaled.load(Ordering::Acquire),
        "{FOREIGN_SEAL_ACCEPTED_DETAIL}"
    );
}
