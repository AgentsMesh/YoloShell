use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::fs::File;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

const CLOSE_ON_EXEC_DETAIL: &str = "passive PTY master descriptor is inheritable";
const DUPLICATE_CONTEXT: &str = "duplicate passive PTY master descriptor";
const MASTER_ACCESS_DETAIL: &str = "passive PTY master has an invalid access mode";
const MASTER_ROLE_CONTEXT: &str = "open exact PTY slave through transferred master";
const SLAVE_DEVICE_DETAIL: &str = "transferred PTY master belongs to another slave device";
const SLAVE_TYPE_DETAIL: &str = "transferred PTY master did not resolve to a character device";
const FIRST_PRIVATE_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const SYSTEM_FAILURE: libc::c_int = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct LinuxPtySlaveIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) rdev: u64,
}

pub(crate) struct LinuxPassivePtyMaster {
    descriptor: OwnedFd,
    slave: LinuxPtySlaveIdentity,
}

impl LinuxPassivePtyMaster {
    pub(crate) fn duplicate_exact(source: RawFd, expected_device: u64) -> Result<Self> {
        let descriptor =
            unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, FIRST_PRIVATE_DESCRIPTOR) };
        ensure!(
            descriptor != SYSTEM_FAILURE,
            std::io::Error::last_os_error()
        );
        Self::from_transferred(unsafe { OwnedFd::from_raw_fd(descriptor) }, expected_device)
            .context(DUPLICATE_CONTEXT)
    }

    pub(crate) fn from_transferred(descriptor: OwnedFd, expected_device: u64) -> Result<Self> {
        validate_descriptor(&descriptor)?;
        let slave = exact_slave_identity(descriptor.as_raw_fd()).context(MASTER_ROLE_CONTEXT)?;
        ensure!(slave.rdev == expected_device, SLAVE_DEVICE_DETAIL);
        Ok(Self { descriptor, slave })
    }

    pub(crate) fn descriptor(&self) -> BorrowedFd<'_> {
        self.descriptor.as_fd()
    }

    pub(crate) fn slave_identity(&self) -> LinuxPtySlaveIdentity {
        self.slave
    }

    pub(crate) fn into_descriptor(self) -> OwnedFd {
        self.descriptor
    }
}

fn validate_descriptor(descriptor: &OwnedFd) -> Result<()> {
    let raw = descriptor.as_raw_fd();
    let descriptor_flags = fcntl(raw, libc::F_GETFD)?;
    ensure!(
        descriptor_flags & libc::FD_CLOEXEC != libc::c_int::default(),
        CLOSE_ON_EXEC_DETAIL
    );
    let status_flags = fcntl(raw, libc::F_GETFL)?;
    ensure!(
        status_flags & libc::O_ACCMODE == libc::O_RDWR,
        MASTER_ACCESS_DETAIL
    );
    Ok(())
}

fn exact_slave_identity(master: RawFd) -> Result<LinuxPtySlaveIdentity> {
    let flags = libc::O_RDONLY | libc::O_NOCTTY | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::ioctl(master, libc::TIOCGPTPEER, flags) };
    ensure!(
        descriptor != SYSTEM_FAILURE,
        std::io::Error::last_os_error()
    );
    let slave = unsafe { File::from_raw_fd(descriptor) };
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    ensure!(
        unsafe { libc::fstat(slave.as_raw_fd(), status.as_mut_ptr()) } != SYSTEM_FAILURE,
        std::io::Error::last_os_error()
    );
    let status = unsafe { status.assume_init() };
    ensure!(
        status.st_mode & libc::S_IFMT == libc::S_IFCHR,
        SLAVE_TYPE_DETAIL
    );
    Ok(LinuxPtySlaveIdentity {
        device: status.st_dev,
        inode: status.st_ino,
        rdev: status.st_rdev,
    })
}

fn fcntl(descriptor: RawFd, operation: libc::c_int) -> Result<libc::c_int> {
    let value = unsafe { libc::fcntl(descriptor, operation) };
    ensure!(value != SYSTEM_FAILURE, std::io::Error::last_os_error());
    Ok(value)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_passive_pty_master_tests.rs"]
mod tests;
