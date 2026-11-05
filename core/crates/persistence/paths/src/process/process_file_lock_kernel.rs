use super::*;
use anyhow::{Context, Result};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};

const MINIMUM_PRIVATE_LEASE_DESCRIPTOR: RawFd = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const FLOCK_SUCCESS: libc::c_int = 0;
const LEASE_DESCRIPTOR_DUPLICATE_CONTEXT: &str = "duplicate process file lock lease descriptor";
const KERNEL_LOCK_CONTEXT: &str = "acquire kernel advisory file lock";

impl ProcessFileLock {
    pub fn try_lock(file: &File, identity: ObjectIdentity, mode: Mode) -> Result<Option<Self>> {
        match registry::begin_kernel(identity, mode)? {
            registry::KernelAcquisition::Acquired(lock) => Ok(Some(lock)),
            registry::KernelAcquisition::Conflict
            | registry::KernelAcquisition::PendingShared(_) => Ok(None),
            registry::KernelAcquisition::Reserved(reservation) => {
                try_reserved(file, mode, reservation)
            }
        }
    }

    pub(crate) fn lock_shared(file: &File, identity: ObjectIdentity) -> Result<Self> {
        loop {
            match registry::begin_kernel(identity, Mode::Shared)? {
                registry::KernelAcquisition::Acquired(lock) => return Ok(lock),
                registry::KernelAcquisition::Conflict => {
                    return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
                        .context(KERNEL_LOCK_CONTEXT);
                }
                registry::KernelAcquisition::PendingShared(waiter) => waiter.wait(),
                registry::KernelAcquisition::Reserved(reservation) => {
                    let descriptor = duplicate_lease_descriptor(file)?;
                    let result = unsafe { libc::flock(descriptor.as_raw_fd(), libc::LOCK_SH) };
                    if result == FLOCK_SUCCESS {
                        return reservation.install(descriptor);
                    }
                    return Err(std::io::Error::last_os_error()).context(KERNEL_LOCK_CONTEXT);
                }
            }
        }
    }
}

fn try_reserved(
    file: &File,
    mode: Mode,
    reservation: registry::KernelReservation,
) -> Result<Option<ProcessFileLock>> {
    let descriptor = duplicate_lease_descriptor(file)?;
    let result = unsafe { libc::flock(descriptor.as_raw_fd(), operation(mode) | libc::LOCK_NB) };
    if result == FLOCK_SUCCESS {
        return reservation.install(descriptor).map(Some);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(error).context(KERNEL_LOCK_CONTEXT)
    }
}

fn duplicate_lease_descriptor(file: &File) -> Result<File> {
    let duplicate = unsafe {
        libc::fcntl(
            file.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            MINIMUM_PRIVATE_LEASE_DESCRIPTOR,
        )
    };
    if duplicate == FCNTL_FAILURE {
        return Err(std::io::Error::last_os_error()).context(LEASE_DESCRIPTOR_DUPLICATE_CONTEXT);
    }
    Ok(unsafe { File::from_raw_fd(duplicate) })
}

fn operation(mode: Mode) -> libc::c_int {
    match mode {
        Mode::Exclusive => libc::LOCK_EX,
        Mode::Shared => libc::LOCK_SH,
    }
}

pub(super) fn unlock(descriptor: RawFd) {
    let _ = unsafe { libc::flock(descriptor, libc::LOCK_UN) };
}
