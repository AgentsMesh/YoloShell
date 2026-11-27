use super::{LinuxContainmentError, LinuxContainmentLease, custody_identity, custody_validation};
use crate::sha256_digest::Sha256Digest;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

const DUPLICATE_FAILURE: libc::c_int = -1;
const FIRST_PRIVATE_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const MISSING_PTY_DEVICE_DETAIL: &str = "containment custody has no bound PTY device";
const UNBOUND_ROOT_DETAIL: &str = "containment custody has no bound root identity";
pub(super) const DESCRIPTOR_COUNT: usize = 2;

pub(crate) struct LinuxContainmentDescriptorExport {
    root: OwnedFd,
    leaf: OwnedFd,
    pty_device: u64,
    recovery_identity_sha256: Sha256Digest,
}

impl LinuxContainmentLease {
    pub(crate) fn export_secondary_custody(
        &self,
    ) -> Result<LinuxContainmentDescriptorExport, LinuxContainmentError> {
        let bound_root = self
            .bound_root
            .ok_or(LinuxContainmentError::Identity(UNBOUND_ROOT_DETAIL))?;
        let pty_device = self
            .record
            .pty_device
            .filter(|device| *device != u64::default())
            .ok_or(LinuxContainmentError::Identity(MISSING_PTY_DEVICE_DETAIL))?;
        let root = File::from(duplicate(&self.root)?);
        let leaf = File::from(duplicate(&self.leaf)?);
        let (root_identity, leaf_identity) = custody_validation::validate(
            &root,
            &leaf,
            &self.record,
            self.record.terminal_id,
            self.record.runtime_epoch,
            bound_root,
        )?;
        let recovery_identity_sha256 =
            custody_identity::capture(&self.record, root_identity, leaf_identity)?;
        Ok(LinuxContainmentDescriptorExport {
            root: root.into(),
            leaf: leaf.into(),
            pty_device,
            recovery_identity_sha256,
        })
    }
}

impl LinuxContainmentDescriptorExport {
    pub(crate) fn root_descriptor(&self) -> BorrowedFd<'_> {
        self.root.as_fd()
    }

    pub(crate) fn leaf_descriptor(&self) -> BorrowedFd<'_> {
        self.leaf.as_fd()
    }

    pub(crate) fn as_descriptors(&self) -> [BorrowedFd<'_>; DESCRIPTOR_COUNT] {
        [self.root_descriptor(), self.leaf_descriptor()]
    }

    pub(crate) fn into_parts(self) -> (OwnedFd, OwnedFd) {
        (self.root, self.leaf)
    }

    pub(crate) fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.recovery_identity_sha256
    }

    pub(crate) fn pty_device(&self) -> u64 {
        self.pty_device
    }
}

fn duplicate(source: &std::fs::File) -> Result<OwnedFd, LinuxContainmentError> {
    let descriptor = unsafe {
        libc::fcntl(
            source.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            FIRST_PRIVATE_DESCRIPTOR,
        )
    };
    if descriptor == DUPLICATE_FAILURE {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
}
