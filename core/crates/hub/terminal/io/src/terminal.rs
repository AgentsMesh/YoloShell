use crate::constants::*;
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
};
use yolo_hub_terminal_protocol::TerminalSize;

pub struct RawTerminal {
    descriptor: OwnedFd,
    original: libc::termios,
}
impl RawTerminal {
    pub fn enable(source: RawFd) -> io::Result<Option<Self>> {
        if !is_terminal(source)? {
            return Ok(None);
        }
        let raw = unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, MIN_DUPLICATE_DESCRIPTOR) };
        if raw == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        // This duplicate pins the terminal descriptor until termios is restored.
        let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(raw, original.as_mut_ptr()) } == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        let original = unsafe { original.assume_init() };
        let mut flags = original;
        unsafe {
            libc::cfmakeraw(&mut flags);
        }
        if unsafe { libc::tcsetattr(raw, libc::TCSANOW, &flags) } == SYSCALL_FAILURE {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(Self {
            descriptor,
            original,
        }))
    }
}
impl Drop for RawTerminal {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.descriptor.as_raw_fd(), libc::TCSANOW, &self.original);
        }
    }
}

pub fn terminal_size(source: RawFd) -> io::Result<Option<TerminalSize>> {
    if !is_terminal(source)? {
        return Ok(None);
    }
    let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
    if unsafe { libc::ioctl(source, libc::TIOCGWINSZ, size.as_mut_ptr()) } == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error());
    }
    let size = unsafe { size.assume_init() };
    if size.ws_col == 0 || size.ws_row == 0 {
        return Ok(None);
    }
    TerminalSize::new(size.ws_col, size.ws_row)
        .map(Some)
        .map_err(|_| io::ErrorKind::InvalidData.into())
}

fn is_terminal(source: RawFd) -> io::Result<bool> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(source, metadata.as_mut_ptr()) } == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error());
    }
    // Darwin reports EOPNOTSUPP for sockets; classify non-character descriptors
    // without weakening errors from actual terminal devices.
    let metadata = unsafe { metadata.assume_init() };
    if metadata.st_mode & libc::S_IFMT != libc::S_IFCHR {
        return Ok(false);
    }
    if unsafe { libc::isatty(source) } == 1 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOTTY) {
        Ok(false)
    } else {
        Err(error)
    }
}
