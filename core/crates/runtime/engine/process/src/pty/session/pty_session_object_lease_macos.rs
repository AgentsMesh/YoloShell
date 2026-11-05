use super::PtySessionProof;
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::{Context, Result, ensure};
use portable_pty::MasterPty;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::Arc;

const MINIMUM_DUPLICATED_FD: libc::c_int = libc::STDERR_FILENO + 1;
const PTY_NAME_CAPACITY: usize = 128;
const SYSCALL_FAILURE: libc::c_int = -1;
const MASTER_FD_UNAVAILABLE_DETAIL: &str = "PTY master descriptor is unavailable";
const MASTER_DUPLICATE_CONTEXT: &str = "duplicate PTY master for object lease";
const MASTER_NAME_CONTEXT: &str = "query PTY slave name from master descriptor";
const MASTER_NAME_INVALID_DETAIL: &str = "PTY master returned an invalid slave name";
const MASTER_PATH_CONTEXT: &str = "canonicalize PTY master slave path";
const MASTER_PATH_MISMATCH_DETAIL: &str = "PTY master belongs to another slave path";
const MASTER_STATUS_CONTEXT: &str = "read PTY master descriptor status";
const MASTER_ACCESS_DETAIL: &str = "PTY master descriptor is not read-write";
const MASTER_METADATA_CONTEXT: &str = "read PTY master slave metadata";
const MASTER_TYPE_DETAIL: &str = "PTY master slave path is not a character device";
const MASTER_IDENTITY_MISMATCH_DETAIL: &str = "PTY master belongs to another PTY object";
const MASTER_LEASE_MISMATCH_DETAIL: &str = "PTY object lease does not match its proof";

#[derive(Clone)]
pub(super) struct MacosPtyObjectLease {
    descriptor: Arc<OwnedFd>,
    path: PathBuf,
    identity: MacosPtyObjectIdentity,
}

impl MacosPtyObjectLease {
    pub(super) fn capture(master: &dyn MasterPty, proof: &PtySessionProof) -> Result<Self> {
        let source = master.as_raw_fd().context(MASTER_FD_UNAVAILABLE_DETAIL)?;
        let descriptor = duplicate_descriptor(source)?;
        let (path, identity) = validate_descriptor(&descriptor, proof)?;
        Ok(Self {
            descriptor: Arc::new(descriptor),
            path,
            identity,
        })
    }

    pub(super) fn duplicate_exact(&self, proof: &PtySessionProof) -> Result<OwnedFd> {
        ensure!(self.matches(proof), MASTER_LEASE_MISMATCH_DETAIL);
        let descriptor = duplicate_descriptor(self.descriptor.as_raw_fd())?;
        validate_descriptor(&descriptor, proof)?;
        Ok(descriptor)
    }

    pub(super) fn matches(&self, proof: &PtySessionProof) -> bool {
        proof.tty_path == self.path && proof.tty_object_identity == Some(self.identity)
    }

    pub(super) fn is_exclusive(&self) -> bool {
        Arc::strong_count(&self.descriptor) == 1
    }
}

pub(super) fn validate_descriptor(
    descriptor: &OwnedFd,
    proof: &PtySessionProof,
) -> Result<(PathBuf, MacosPtyObjectIdentity)> {
    let (path, identity) = inspect_descriptor(descriptor)?;
    ensure!(path == proof.tty_path, MASTER_PATH_MISMATCH_DETAIL);
    ensure!(
        proof.tty_object_identity == Some(identity),
        MASTER_IDENTITY_MISMATCH_DETAIL
    );
    Ok((path, identity))
}

pub(super) fn inspect_descriptor(
    descriptor: &OwnedFd,
) -> Result<(PathBuf, MacosPtyObjectIdentity)> {
    inspect_raw_descriptor(descriptor.as_raw_fd())
}

pub(in crate::process) fn inspect_output_pty_identity(
    descriptor: &File,
) -> Result<MacosPtyObjectIdentity> {
    inspect_raw_descriptor(descriptor.as_raw_fd()).map(|(_path, identity)| identity)
}

fn inspect_raw_descriptor(descriptor: libc::c_int) -> Result<(PathBuf, MacosPtyObjectIdentity)> {
    use std::os::unix::fs::FileTypeExt;
    let status = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(status != SYSCALL_FAILURE, MASTER_STATUS_CONTEXT);
    ensure!(
        status & libc::O_ACCMODE == libc::O_RDWR,
        MASTER_ACCESS_DETAIL
    );
    let path = slave_path(descriptor)?;
    let metadata = std::fs::metadata(&path).context(MASTER_METADATA_CONTEXT)?;
    ensure!(metadata.file_type().is_char_device(), MASTER_TYPE_DETAIL);
    let identity = MacosPtyObjectIdentity::from_metadata(&metadata);
    Ok((path, identity))
}

fn duplicate_descriptor(source: libc::c_int) -> Result<OwnedFd> {
    let duplicate = unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, MINIMUM_DUPLICATED_FD) };
    if duplicate == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(MASTER_DUPLICATE_CONTEXT);
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

fn slave_path(descriptor: libc::c_int) -> Result<PathBuf> {
    let mut name = [u8::default(); PTY_NAME_CAPACITY];
    let result = unsafe { libc::ioctl(descriptor, libc::TIOCPTYGNAME.into(), name.as_mut_ptr()) };
    if result == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(MASTER_NAME_CONTEXT);
    }
    let length = name
        .iter()
        .position(|byte| *byte == u8::default())
        .context(MASTER_NAME_INVALID_DETAIL)?;
    ensure!(length != usize::default(), MASTER_NAME_INVALID_DETAIL);
    let path = PathBuf::from(std::ffi::OsStr::from_bytes(&name[..length]));
    std::fs::canonicalize(path).context(MASTER_PATH_CONTEXT)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/object/pty_session_object_lease_macos_tests.rs"]
mod tests;
