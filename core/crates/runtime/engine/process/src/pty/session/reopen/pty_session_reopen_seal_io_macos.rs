use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

#[path = "pty_session_reopen_seal_io_hooks_macos.rs"]
mod test_hooks;
#[cfg(not(any(test, feature = "runtime-component-tests")))]
use test_hooks::TestIoOperation;
use test_hooks::injected_failure;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use test_hooks::{TestIoOperation, fail_next, fail_repeated_after_next};

const SYSCALL_FAILURE: libc::c_int = -1;
const SLAVE_OPEN_FLAGS: libc::c_int =
    libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const PERMISSION_BITS: libc::mode_t =
    (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as libc::mode_t;
const SLAVE_PATH_CONTEXT: &str = "encode exact PTY slave path";
const SLAVE_OPEN_CONTEXT: &str = "open exact PTY slave for reopen seal";
const SLAVE_STATUS_CONTEXT: &str = "inspect exact PTY slave for reopen seal";
const SLAVE_CHMOD_CONTEXT: &str = "change exact PTY slave mode for reopen seal";
const SLAVE_PATH_CHMOD_CONTEXT: &str = "change exact no-follow PTY slave path mode";
const SLAVE_PATH_STATUS_CONTEXT: &str = "inspect sealed PTY slave path";
const SLAVE_TYPE_DETAIL: &str = "reopen-seal target is not a character device";
const SLAVE_IDENTITY_DETAIL: &str = "reopen-seal PTY object identity changed";
const SLAVE_MODE_DETAIL: &str = "reopen-seal PTY mode change was not committed";

pub(super) struct SlaveStatus {
    pub(super) identity: MacosPtyObjectIdentity,
    pub(super) mode: libc::mode_t,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ExactPathSealState {
    Retired,
    Sealed,
    RepairRequired,
}

pub(super) fn open_slave(path: &Path) -> Result<OwnedFd> {
    injected_failure(TestIoOperation::OpenSlave)?;
    let path = c_path(path)?;
    let descriptor = unsafe { libc::open(path.as_ptr(), SLAVE_OPEN_FLAGS) };
    if descriptor == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(SLAVE_OPEN_CONTEXT);
    }
    Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
}

pub(super) fn descriptor_status(descriptor: &OwnedFd) -> Result<SlaveStatus> {
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe { libc::fstat(descriptor.as_raw_fd(), status.as_mut_ptr()) };
    if result == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(SLAVE_STATUS_CONTEXT);
    }
    let status = unsafe { status.assume_init() };
    ensure!(
        status.st_mode & libc::S_IFMT == libc::S_IFCHR,
        SLAVE_TYPE_DETAIL
    );
    Ok(SlaveStatus {
        identity: MacosPtyObjectIdentity::from_stat(&status),
        mode: status.st_mode & PERMISSION_BITS,
    })
}

pub(super) fn set_descriptor_mode(descriptor: &OwnedFd, mode: libc::mode_t) -> Result<()> {
    injected_failure(TestIoOperation::ChangeMode)?;
    set_descriptor_mode_fail_closed(descriptor, mode)
}

pub(super) fn set_descriptor_mode_fail_closed(
    descriptor: &OwnedFd,
    mode: libc::mode_t,
) -> Result<()> {
    let result = unsafe { libc::fchmod(descriptor.as_raw_fd(), mode) };
    if result == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(SLAVE_CHMOD_CONTEXT);
    }
    Ok(())
}

pub(super) fn set_path_mode(path: &Path, mode: libc::mode_t) -> Result<()> {
    injected_failure(TestIoOperation::ChangeMode)?;
    set_path_mode_fail_closed(path, mode)
}

pub(super) fn set_path_mode_fail_closed(path: &Path, mode: libc::mode_t) -> Result<()> {
    let path = c_path(path)?;
    let result = unsafe {
        libc::fchmodat(
            libc::AT_FDCWD,
            path.as_ptr(),
            mode,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(SLAVE_PATH_CHMOD_CONTEXT);
    }
    Ok(())
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn validate_seal_commit() -> Result<()> {
    injected_failure(TestIoOperation::ValidateSeal)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) fn finalize_quarantine_for_test() -> Result<()> {
    injected_failure(TestIoOperation::FinalizeQuarantine)
}

pub(super) fn validate_path(
    path: &Path,
    identity: MacosPtyObjectIdentity,
    mode: libc::mode_t,
) -> Result<()> {
    injected_failure(TestIoOperation::ValidateSeal)?;
    validate_path_exact(path, identity, mode)
}

pub(super) fn validate_path_exact(
    path: &Path,
    identity: MacosPtyObjectIdentity,
    mode: libc::mode_t,
) -> Result<()> {
    let status = path_status(path)?.context(SLAVE_IDENTITY_DETAIL)?;
    ensure!(status.identity == identity, SLAVE_IDENTITY_DETAIL);
    ensure!(status.mode == mode, SLAVE_MODE_DETAIL);
    Ok(())
}

pub(super) fn exact_path_is_sealed(
    path: &Path,
    identity: MacosPtyObjectIdentity,
    sealed_mode: libc::mode_t,
) -> Result<bool> {
    Ok(exact_path_seal_state(path, identity, sealed_mode)? == ExactPathSealState::Sealed)
}

pub(super) fn exact_path_seal_state(
    path: &Path,
    identity: MacosPtyObjectIdentity,
    sealed_mode: libc::mode_t,
) -> Result<ExactPathSealState> {
    let Some(status) = path_status(path)? else {
        return Ok(ExactPathSealState::Retired);
    };
    ensure!(status.identity == identity, SLAVE_IDENTITY_DETAIL);
    Ok(if status.mode == sealed_mode {
        ExactPathSealState::Sealed
    } else {
        ExactPathSealState::RepairRequired
    })
}

fn path_status(path: &Path) -> Result<Option<SlaveStatus>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context(SLAVE_PATH_STATUS_CONTEXT),
    };
    ensure!(metadata.file_type().is_char_device(), SLAVE_TYPE_DETAIL);
    use std::os::unix::fs::PermissionsExt;
    Ok(Some(SlaveStatus {
        identity: MacosPtyObjectIdentity::from_metadata(&metadata),
        mode: metadata.permissions().mode() as libc::mode_t & PERMISSION_BITS,
    }))
}

fn c_path(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_bytes()).context(SLAVE_PATH_CONTEXT)
}
