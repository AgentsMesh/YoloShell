use super::cleanup_connect::{ConnectSocket, ConnectStart, ConnectWait};
use std::io;
use std::mem::{offset_of, size_of, size_of_val};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "cleanup_connect_native_config.rs"]
mod config;
use config::configure_socket;

const CONNECT_PATH_NUL_DETAIL: &str = "cleanup socket path contains a NUL byte";
const CONNECT_PATH_TOO_LONG_DETAIL: &str = "cleanup socket path exceeds sockaddr_un capacity";
const CONNECT_ADDRESS_LENGTH_DETAIL: &str = "cleanup socket address exceeds platform range";
const CONNECT_POLL_INVALID_DETAIL: &str = "cleanup socket descriptor became invalid during poll";
const DEFAULT_SOCKET_PROTOCOL: libc::c_int = 0;
const INVALID_DESCRIPTOR: RawFd = -1;
const NO_SOCKET_ERROR: libc::c_int = 0;
const NO_POLL_EVENTS: libc::c_short = 0;
const NUL_BYTE: u8 = 0;
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
const POLL_TIMED_OUT: libc::c_int = 0;
const MINIMUM_POLL_MILLISECONDS: u128 = 1;
const PATH_TERMINATOR_BYTES: usize = 1;
const SYSCALL_SUCCESS: libc::c_int = 0;

pub(super) fn open_socket() -> io::Result<ConnectSocket> {
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, DEFAULT_SOCKET_PROTOCOL) };
    if raw == INVALID_DESCRIPTOR {
        return Err(io::Error::last_os_error());
    }
    let descriptor = unsafe { OwnedFd::from_raw_fd(raw) };
    configure_socket(raw)?;
    let descriptor_flags = fcntl_get(raw, libc::F_GETFD)?;
    fcntl_set(raw, libc::F_SETFD, descriptor_flags | libc::FD_CLOEXEC)?;
    let original_status = fcntl_get(raw, libc::F_GETFL)?;
    fcntl_set(raw, libc::F_SETFL, original_status | libc::O_NONBLOCK)?;
    Ok(ConnectSocket {
        descriptor,
        original_status,
    })
}

pub(super) fn start(socket: &ConnectSocket, path: &Path) -> io::Result<ConnectStart> {
    let (address, length) = unix_address(path)?;
    let result = unsafe {
        libc::connect(
            socket.descriptor.as_raw_fd(),
            (&raw const address).cast::<libc::sockaddr>(),
            length,
        )
    };
    if result == SYSCALL_SUCCESS {
        return Ok(ConnectStart::Connected);
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::EINPROGRESS) | Some(libc::EALREADY) => Ok(ConnectStart::Pending),
        _ if error.kind() == io::ErrorKind::WouldBlock => Ok(ConnectStart::Pending),
        _ => Err(error),
    }
}

pub(super) fn wait(socket: &ConnectSocket, timeout: Duration) -> io::Result<ConnectWait> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(ConnectWait::TimedOut);
        }
        match poll_writable(socket.descriptor.as_raw_fd(), remaining) {
            Ok(ConnectWait::TimedOut) if Instant::now() < deadline => continue,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

pub(super) fn socket_error(socket: &ConnectSocket) -> io::Result<libc::c_int> {
    let mut error = NO_SOCKET_ERROR;
    let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>()).unwrap();
    let result = unsafe {
        libc::getsockopt(
            socket.descriptor.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_ERROR,
            (&raw mut error).cast(),
            &mut length,
        )
    };
    if result == SYSCALL_SUCCESS {
        Ok(error)
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn restore_blocking(socket: &ConnectSocket) -> io::Result<()> {
    fcntl_set(
        socket.descriptor.as_raw_fd(),
        libc::F_SETFL,
        socket.original_status,
    )
}

fn unix_address(path: &Path) -> io::Result<(libc::sockaddr_un, libc::socklen_t)> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.contains(&NUL_BYTE) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            CONNECT_PATH_NUL_DETAIL,
        ));
    }
    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_un>() };
    if bytes.len().saturating_add(PATH_TERMINATOR_BYTES) > size_of_val(&address.sun_path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            CONNECT_PATH_TOO_LONG_DETAIL,
        ));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as libc::c_char;
    }
    let length = offset_of!(libc::sockaddr_un, sun_path)
        .saturating_add(bytes.len())
        .saturating_add(PATH_TERMINATOR_BYTES);
    let length = libc::socklen_t::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, CONNECT_ADDRESS_LENGTH_DETAIL))?;
    #[cfg(target_os = "macos")]
    {
        address.sun_len = u8::try_from(length).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, CONNECT_ADDRESS_LENGTH_DETAIL)
        })?;
    }
    Ok((address, length))
}

fn poll_writable(descriptor: RawFd, timeout: Duration) -> io::Result<ConnectWait> {
    let timeout_ms = timeout.as_millis().max(MINIMUM_POLL_MILLISECONDS);
    let timeout_ms = libc::c_int::try_from(timeout_ms).unwrap_or(libc::c_int::MAX);
    let mut poll = libc::pollfd {
        fd: descriptor,
        events: libc::POLLOUT,
        revents: NO_POLL_EVENTS,
    };
    let result = unsafe { libc::poll(&mut poll, POLL_DESCRIPTOR_COUNT, timeout_ms) };
    if result == INVALID_DESCRIPTOR {
        return Err(io::Error::last_os_error());
    }
    if result == POLL_TIMED_OUT {
        return Ok(ConnectWait::TimedOut);
    }
    if poll.revents & libc::POLLNVAL != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            CONNECT_POLL_INVALID_DETAIL,
        ));
    }
    Ok(ConnectWait::Ready)
}

fn fcntl_get(descriptor: RawFd, command: libc::c_int) -> io::Result<libc::c_int> {
    let result = unsafe { libc::fcntl(descriptor, command) };
    (result != INVALID_DESCRIPTOR)
        .then_some(result)
        .ok_or_else(io::Error::last_os_error)
}

fn fcntl_set(descriptor: RawFd, command: libc::c_int, flags: libc::c_int) -> io::Result<()> {
    let result = unsafe { libc::fcntl(descriptor, command, flags) };
    (result != INVALID_DESCRIPTOR)
        .then_some(())
        .ok_or_else(io::Error::last_os_error)
}
