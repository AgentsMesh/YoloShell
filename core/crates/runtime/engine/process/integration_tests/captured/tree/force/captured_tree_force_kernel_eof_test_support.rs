use crate::process::captured_tree::force::KernelEofKillReady;
use crate::process::kernel_eof_authority::{
    KernelPtyEofIdentity, ValidatedKernelEof, bind_scripted_kernel_eof,
};
use crate::process::{FrozenProcessTree, KernelEofKillPreparation};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;

const SLAVE_OPEN_FLAGS: libc::c_int =
    libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const PERMISSION_BITS: u32 = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as u32;
const TEST_WRITE: &[u8] = b"reopen-after-kernel-eof";
const CAPABILITY_PROCESS_SEPARATOR: &str = "-process-";
const REOPEN_ALLOWED_DETAIL: &str = "PTY slave reopened after the final frozen inspection";
const FIRST_EOF_REUSED_DETAIL: &str = "first kernel EOF was reused after acquiring a slave";
const EOF_IDENTITY_REUSED_DETAIL: &str = "fresh kernel EOF reused the stale EOF identity";
pub(super) const PREPARATION_REFROZE_DETAIL: &str =
    "stable exact closure refroze before reopen sealing";
pub(super) const SEALED_MODE: u32 = 0;

pub(super) fn scoped_capability(base: &str) -> String {
    format!("{base}{CAPABILITY_PROCESS_SEPARATOR}{}", std::process::id())
}

pub(super) fn assert_reopen_denied(path: &Path) {
    let opened = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(path);
    if let Ok(mut slave) = opened {
        let _ = slave.write_all(TEST_WRITE);
        panic!("{REOPEN_ALLOWED_DETAIL}");
    }
}

pub(super) fn prepare_after_fresh_eof(
    frozen: &mut FrozenProcessTree,
) -> (KernelEofKillReady, ValidatedKernelEof) {
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
    let fresh = KernelPtyEofIdentity::from_kernel_read();
    assert!(!Arc::ptr_eq(&stale, &fresh), "{EOF_IDENTITY_REUSED_DETAIL}");
    let (permit, validation) = bind_scripted_kernel_eof(&version, &fresh);
    frozen.install_kernel_eof_authority_for_test(permit);
    let KernelEofKillPreparation::Ready(ready) = frozen
        .prepare_kill_with_kernel_eof(version.clone())
        .unwrap()
    else {
        panic!("{PREPARATION_REFROZE_DETAIL}");
    };
    let validated = validation.validate_scripted(&version, &fresh).unwrap();
    (ready, validated)
}

pub(super) fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS
}
