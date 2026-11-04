use crate::constants::*;
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    sync::Arc,
};

struct Entry {
    descriptor: OwnedFd,
    original_flags: libc::c_int,
}

struct Restoration {
    entries: Vec<Entry>,
}
impl Drop for Restoration {
    fn drop(&mut self) {
        for entry in &self.entries {
            unsafe {
                libc::fcntl(
                    entry.descriptor.as_raw_fd(),
                    libc::F_SETFL,
                    entry.original_flags,
                );
            }
        }
    }
}

pub(crate) struct Registered {
    restoration: Arc<Restoration>,
    index: usize,
}
impl AsRawFd for Registered {
    fn as_raw_fd(&self) -> RawFd {
        self.restoration.entries[self.index].descriptor.as_raw_fd()
    }
}

pub(crate) fn capture(sources: &[RawFd]) -> io::Result<Vec<Registered>> {
    let mut restoration = Restoration {
        entries: Vec::with_capacity(sources.len()),
    };
    // Capture every original flag before changing any open-file description. The
    // group also defers restoration until all possibly-aliased users have stopped.
    for source in sources {
        let raw = unsafe { libc::fcntl(*source, libc::F_DUPFD_CLOEXEC, MIN_DUPLICATE_DESCRIPTOR) };
        if raw == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
        let original_flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
        if original_flags == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        restoration.entries.push(Entry {
            descriptor,
            original_flags,
        });
    }
    for entry in &restoration.entries {
        if unsafe {
            libc::fcntl(
                entry.descriptor.as_raw_fd(),
                libc::F_SETFL,
                entry.original_flags | libc::O_NONBLOCK,
            )
        } == SYSCALL_FAILURE
        {
            return Err(io::Error::last_os_error());
        }
    }
    let restoration = Arc::new(restoration);
    Ok((0..sources.len())
        .map(|index| Registered {
            restoration: restoration.clone(),
            index,
        })
        .collect())
}
