use super::exact_gate_fixture::ExactGateFixture;
use super::kernel_eof_test_support::{
    SEALED_MODE, assert_reopen_denied, mode, prepare_after_fresh_eof, scoped_capability,
};
use crate::process::{FrozenProcessTree, install_test_signal_hook};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const TAMPERED_SEAL_CAPABILITY: &str = "kernel-eof-seal-validation-tamper";
const QUARANTINE_CAPABILITY: &str = "kernel-eof-seal-validation-quarantine";
const FIRST_SIGNAL_TARGET: usize = 0;
const TAMPERED_MODE: u32 = (libc::S_IRUSR | libc::S_IWUSR) as u32;
const TAMPERED_SEAL_ACCEPTED_DETAIL: &str = "tampered committed seal was accepted";
const TAMPERED_SEAL_SIGNALED_DETAIL: &str = "tampered committed seal authorized SIGKILL";
const RECOVERY_FAILURE_IGNORED_DETAIL: &str = "persistent reopen-seal recovery failure was ignored";
const RECOVERY_NOT_QUARANTINED_DETAIL: &str =
    "exhausted reopen-seal recovery retained active descriptor custody";
const PHYSICAL_SEAL_MISSING_DETAIL: &str = "reopen-seal cleanup did not restore exact PTY mode";

#[test]
fn tampered_committed_seal_cannot_reach_sigkill() {
    let capability = scoped_capability(TAMPERED_SEAL_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let (prepared, validated) = prepare_after_fresh_eof(&mut frozen);
    set_mode(&path, TAMPERED_MODE);
    let validated = frozen.seal_kernel_eof_for_test(&prepared, validated);
    let signaled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&signaled);
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        observed.store(true, Ordering::Release);
    });

    let error = match frozen.commit_kill_with_kernel_eof(prepared, validated) {
        Ok(_) => panic!("{TAMPERED_SEAL_ACCEPTED_DETAIL}"),
        Err(error) => error,
    };
    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
    assert!(
        !signaled.load(Ordering::Acquire),
        "{TAMPERED_SEAL_SIGNALED_DETAIL}"
    );
    assert_eq!(mode(&path), SEALED_MODE, "{PHYSICAL_SEAL_MISSING_DETAIL}");
    assert_reopen_denied(&path);
}

#[test]
fn persistent_cleanup_failure_quarantines_reopen_seal_without_signaling() {
    let capability = scoped_capability(QUARANTINE_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let (prepared, validated) = prepare_after_fresh_eof(&mut frozen);
    set_mode(&path, TAMPERED_MODE);
    let validated = frozen.seal_kernel_eof_for_test(&prepared, validated);
    let signaled = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&signaled);
    let _hook = install_test_signal_hook(libc::SIGKILL, FIRST_SIGNAL_TARGET, move || {
        observed.store(true, Ordering::Release);
    });
    let _failures = crate::process::pty_session_proof::fail_reopen_seal_recovery_for_test();

    let error = match frozen.commit_kill_with_kernel_eof(prepared, validated) {
        Ok(_) => panic!("{RECOVERY_FAILURE_IGNORED_DETAIL}"),
        Err(error) => error,
    };

    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
    assert!(
        frozen.reopen_seal_is_quarantined_for_test(),
        "{RECOVERY_NOT_QUARANTINED_DETAIL}"
    );
    assert!(
        !signaled.load(Ordering::Acquire),
        "{TAMPERED_SEAL_SIGNALED_DETAIL}"
    );
    assert_eq!(mode(&path), SEALED_MODE, "{PHYSICAL_SEAL_MISSING_DETAIL}");
    assert_reopen_denied(&path);
}

fn set_mode(path: &std::path::Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
