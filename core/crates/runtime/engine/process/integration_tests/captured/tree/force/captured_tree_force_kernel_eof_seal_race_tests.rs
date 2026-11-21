use super::exact_gate_fixture::ExactGateFixture;
use super::kernel_eof_test_support::{assert_reopen_denied, scoped_capability};
use crate::process::KernelEofKillPreparation;
use crate::process::kernel_eof_authority::{KernelPtyEofIdentity, bind_scripted_kernel_eof};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;

const PRE_SEAL_RACE_CAPABILITY: &str = "kernel-eof-reopen-seal-pre-seal-race";
const POST_SEAL_RACE_CAPABILITY: &str = "kernel-eof-reopen-seal-post-seal-race";
const SLAVE_OPEN_FLAGS: libc::c_int =
    libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const PRE_SEAL_REFREEZE_DETAIL: &str = "pre-seal closure change established a reopen seal";
const FIRST_EOF_REUSED_DETAIL: &str = "first kernel EOF survived reopen-seal acquisition";
const EOF_IDENTITY_REUSED_DETAIL: &str = "fresh kernel EOF reused the stale allocation";
const POST_SEAL_REFREEZE_DETAIL: &str = "post-seal closure change bypassed refreeze";

#[test]
fn pre_seal_closure_change_refreezes_without_establishing_a_seal() {
    let capability = scoped_capability(PRE_SEAL_RACE_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let source = frozen.tree.captured_targets()[usize::default()].0;
    frozen.tree.events.inject_unresolved_fork(source);

    let prepared = frozen
        .prepare_kill_with_kernel_eof(frozen.closure_version())
        .unwrap();

    assert!(matches!(prepared, KernelEofKillPreparation::Refreeze));
    assert!(
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(SLAVE_OPEN_FLAGS)
            .open(path)
            .is_ok(),
        "{PRE_SEAL_REFREEZE_DETAIL}"
    );
}

#[test]
fn post_seal_closure_change_refreezes_with_fresh_eof() {
    let capability = scoped_capability(POST_SEAL_RACE_CAPABILITY);
    let mut fixture = ExactGateFixture::spawn(&capability, false);
    let path = fixture.slave_path();
    let mut frozen = fixture.take_kernel_eof_frozen();
    let version = frozen.closure_version();
    let stale = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&version, &stale);
    frozen.install_kernel_eof_authority_for_test(permit);

    assert!(
        matches!(
            frozen
                .prepare_kill_with_kernel_eof(version.clone())
                .unwrap(),
            KernelEofKillPreparation::FreshEofRequired
        ),
        "{FIRST_EOF_REUSED_DETAIL}"
    );
    assert_reopen_denied(&path);

    let fresh = KernelPtyEofIdentity::from_kernel_read();
    assert!(!Arc::ptr_eq(&stale, &fresh), "{EOF_IDENTITY_REUSED_DETAIL}");
    let (permit, _) = bind_scripted_kernel_eof(&version, &fresh);
    frozen.install_kernel_eof_authority_for_test(permit);
    let source = frozen.tree.captured_targets()[usize::default()].0;
    frozen.tree.events.inject_unresolved_fork(source);

    assert!(
        matches!(
            frozen.prepare_kill_with_kernel_eof(version).unwrap(),
            KernelEofKillPreparation::Refreeze
        ),
        "{POST_SEAL_REFREEZE_DETAIL}"
    );
    assert_reopen_denied(&path);
}
