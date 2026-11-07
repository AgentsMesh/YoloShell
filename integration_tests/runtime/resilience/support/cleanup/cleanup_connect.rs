use super::cleanup_connect_native;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

const CONNECT_TIMEOUT_DETAIL: &str = "cleanup socket connect timed out";
const NO_SOCKET_ERROR: libc::c_int = 0;

pub(super) fn connect(path: &Path, timeout: Duration) -> io::Result<UnixStream> {
    connect_with(path, timeout, &NativeBackend)
}

pub(super) enum ConnectStart {
    Connected,
    Pending,
}

pub(super) enum ConnectWait {
    Ready,
    TimedOut,
}

pub(super) struct ConnectSocket {
    pub(super) descriptor: OwnedFd,
    pub(super) original_status: libc::c_int,
}

pub(super) trait ConnectBackend {
    fn open(&self) -> io::Result<ConnectSocket>;
    fn start(&self, socket: &ConnectSocket, path: &Path) -> io::Result<ConnectStart>;
    fn wait(&self, socket: &ConnectSocket, timeout: Duration) -> io::Result<ConnectWait>;
    fn socket_error(&self, socket: &ConnectSocket) -> io::Result<libc::c_int>;
    fn restore_blocking(&self, socket: &ConnectSocket) -> io::Result<()>;
}

pub(super) fn connect_with(
    path: &Path,
    timeout: Duration,
    backend: &impl ConnectBackend,
) -> io::Result<UnixStream> {
    let socket = backend.open()?;
    if matches!(backend.start(&socket, path)?, ConnectStart::Pending) {
        if matches!(backend.wait(&socket, timeout)?, ConnectWait::TimedOut) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                CONNECT_TIMEOUT_DETAIL,
            ));
        }
        let error = backend.socket_error(&socket)?;
        if error != NO_SOCKET_ERROR {
            return Err(io::Error::from_raw_os_error(error));
        }
    }
    backend.restore_blocking(&socket)?;
    Ok(socket.descriptor.into())
}

struct NativeBackend;

impl ConnectBackend for NativeBackend {
    fn open(&self) -> io::Result<ConnectSocket> {
        cleanup_connect_native::open_socket()
    }

    fn start(&self, socket: &ConnectSocket, path: &Path) -> io::Result<ConnectStart> {
        cleanup_connect_native::start(socket, path)
    }

    fn wait(&self, socket: &ConnectSocket, timeout: Duration) -> io::Result<ConnectWait> {
        cleanup_connect_native::wait(socket, timeout)
    }

    fn socket_error(&self, socket: &ConnectSocket) -> io::Result<libc::c_int> {
        cleanup_connect_native::socket_error(socket)
    }

    fn restore_blocking(&self, socket: &ConnectSocket) -> io::Result<()> {
        cleanup_connect_native::restore_blocking(socket)
    }
}
