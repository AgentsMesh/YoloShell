use super::exact_gate_fixture::ExactGateFixture;
use super::kernel_eof_test_support::{
    PREPARATION_REFROZE_DETAIL, SEALED_MODE, assert_reopen_denied, mode, prepare_after_fresh_eof,
    scoped_capability,
};
use crate::process::captured_targets::{install_test_signal_failure, install_test_signal_noop};
use crate::process::{KernelEofKillPreparation, KillCommitReady, install_test_signal_hook};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const EXACT_POSITIVE_CAPABILITY: &str = "kernel-eof-reopen-seal-exact-positive";
const ORDINARY_KILL_CAPABILITY: &str = "kernel-eof-reopen-seal-ordinary-kill";
const FINAL_INSPECTION_CAPABILITY: &str = "kernel-eof-reopen-seal-final-inspection";
const SIGNAL_FAILURE_CAPABILITY: &str = "kernel-eof-reopen-seal-signal-failure";
const FIRST_SIGNAL_TARGET: usize = 0;
const SECOND_SIGNAL_TARGET: usize = 1;
const SIGNAL_HOOK_MISSING_DETAIL: &str = "kernel EOF commit did not reach SIGKILL";
const MODE_RESTORED_EARLY_DETAIL: &str = "committed process custody restored PTY mode too early";
const SIGNAL_FAILURE_DETAIL: &str = "injected kernel EOF SIGKILL failure";
const SIGNAL_FAILURE_IGNORED_DETAIL: &str = "kernel EOF SIGKILL failure was ignored";
const EXACT_SEAL_MISSING_DETAIL: &str = "exact SIGKILL ran before PTY reopen was sealed";
const ORDINARY_SEAL_MISSING_DETAIL: &str = "ordinary SIGKILL ran before PTY reopen was sealed";
const PREMATURE_SIGNAL_DETAIL: &str = "first kernel EOF transaction reached SIGKILL";

#[test]
fn exact_positive_seals_reopen_before_sigkill() {
    let capability = scoped_capability(EXACT_POSITIVE_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_frozen();
    let observed = Arc::new(AtomicBool::new(false));
    let signaled = Arc::clone(&observed);
    let path_at_signal = path.clone();
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        assert_reopen_denied(&path_at_signal);
        signaled.store(true, Ordering::Release);
    });

    let version = frozen.closure_version();
    let ready = frozen.prepare_kill(version).unwrap();

    assert!(matches!(
        ready,
        KillCommitReady::PendingExact(_) | KillCommitReady::PendingIncremental(_)
    ));
    assert!(
        observed.load(Ordering::Acquire),
        "{EXACT_SEAL_MISSING_DETAIL}"
    );
    assert_reopen_denied(&path);
}

#[test]
fn ordinary_kill_seals_reopen_before_sigkill() {
    let capability = scoped_capability(ORDINARY_KILL_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_ordinary_frozen();
    let observed = Arc::new(AtomicBool::new(false));
    let signaled = Arc::clone(&observed);
    let path_at_signal = path.clone();
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        assert_reopen_denied(&path_at_signal);
        signaled.store(true, Ordering::Release);
    });

    let version = frozen.closure_version();
    let ready = frozen.prepare_kill(version).unwrap();

    assert!(matches!(ready, KillCommitReady::PendingOrdinary(_)));
    assert!(
        observed.load(Ordering::Acquire),
        "{ORDINARY_SEAL_MISSING_DETAIL}"
    );
    assert_reopen_denied(&path);
}

#[test]
fn final_frozen_inspection_seals_reopen_before_sigkill() {
    let capability = scoped_capability(FINAL_INSPECTION_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let signaled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&signaled);
    let path_at_signal = path.clone();
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        assert_reopen_denied(&path_at_signal);
        observed.store(true, Ordering::Release);
    });
    let (prepared, validated) = prepare_after_fresh_eof(&mut frozen);
    assert!(
        !signaled.load(Ordering::Acquire),
        "{PREMATURE_SIGNAL_DETAIL}"
    );
    let validated = frozen.seal_kernel_eof_for_test(&prepared, validated);
    let ready = frozen
        .commit_kill_with_kernel_eof(prepared, validated)
        .unwrap();

    assert!(matches!(ready, KillCommitReady::PendingKernelEof(_)));
    assert!(
        signaled.load(Ordering::Acquire),
        "{SIGNAL_HOOK_MISSING_DETAIL}"
    );
    assert_reopen_denied(&path);
    drop(frozen);
    assert_eq!(mode(&path), SEALED_MODE, "{MODE_RESTORED_EARLY_DETAIL}");
}

#[test]
fn committed_seal_is_reused_after_signal_failure() {
    let capability = scoped_capability(SIGNAL_FAILURE_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let (prepared, validated) = prepare_after_fresh_eof(&mut frozen);
    let failure =
        install_test_signal_failure(libc::SIGKILL, FIRST_SIGNAL_TARGET, SIGNAL_FAILURE_DETAIL);
    let noop = install_test_signal_noop(libc::SIGKILL, SECOND_SIGNAL_TARGET);

    let validated = frozen.seal_kernel_eof_for_test(&prepared, validated);
    assert!(
        frozen
            .commit_kill_with_kernel_eof(prepared, validated)
            .is_err(),
        "{SIGNAL_FAILURE_IGNORED_DETAIL}"
    );
    drop((failure, noop));
    assert_reopen_denied(&path);

    let prepared = frozen
        .prepare_kill_with_kernel_eof(frozen.closure_version())
        .unwrap();
    let KernelEofKillPreparation::Ready(prepared) = prepared else {
        panic!("{PREPARATION_REFROZE_DETAIL}");
    };
    let validated = frozen.validated_kernel_eof_for_test();
    let validated = frozen.seal_kernel_eof_for_test(&prepared, validated);
    let ready = frozen
        .commit_kill_with_kernel_eof(prepared, validated)
        .unwrap();

    assert!(matches!(ready, KillCommitReady::PendingKernelEof(_)));
    assert_reopen_denied(&path);
}
