use anyhow::{Context, Result, ensure};
use std::{
    mem::{offset_of, size_of},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
    path::Path,
};

const OPEN_SOCKET: &str = "open bounded unbound endpoint probe";
const CONFIGURE_SOCKET: &str = "configure nonblocking unbound endpoint probe";
const INVALID_ADDRESS: &str = "unbound endpoint probe path is invalid";
const NULL_TERMINATOR_BYTES: usize = 1;

/// No blocking wait and no ambiguous absence inference: only ECONNREFUSED proves
/// a stale listener. Success, EINPROGRESS, EAGAIN and every other outcome refuse recovery.
pub(super) fn refused(path: &Path) -> Result<bool> {
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    ensure!(
        raw >= 0,
        "{OPEN_SOCKET}: {}",
        std::io::Error::last_os_error()
    );
    let socket = unsafe { OwnedFd::from_raw_fd(raw) };
    let flags = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFL) };
    ensure!(flags >= 0, CONFIGURE_SOCKET);
    ensure!(
        unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } == 0,
        CONFIGURE_SOCKET
    );
    ensure!(
        unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == 0,
        CONFIGURE_SOCKET
    );
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    let path = path.as_os_str().as_bytes();
    ensure!(
        !path.is_empty() && !path.contains(&0) && path.len() < address.sun_path.len(),
        INVALID_ADDRESS
    );
    address.sun_family = libc::AF_UNIX as _;
    for (target, source) in address.sun_path.iter_mut().zip(path) {
        *target = *source as libc::c_char;
    }
    let length = offset_of!(libc::sockaddr_un, sun_path) + path.len() + NULL_TERMINATOR_BYTES;
    ensure!(length <= size_of::<libc::sockaddr_un>(), INVALID_ADDRESS);
    #[cfg(target_os = "macos")]
    {
        address.sun_len = u8::try_from(length).context(INVALID_ADDRESS)?;
    }
    let result = unsafe {
        libc::connect(
            socket.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            libc::socklen_t::try_from(length).context(INVALID_ADDRESS)?,
        )
    };
    Ok(result < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECONNREFUSED))
}
