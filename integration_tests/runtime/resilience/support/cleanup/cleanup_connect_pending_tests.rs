use super::*;
use std::cell::RefCell;
use std::io::{self, Read};
use std::os::fd::OwnedFd;

const FIXTURE_ORIGINAL_STATUS: libc::c_int = 0;
const FIXTURE_FAILURE_DETAIL: &str = "injected cleanup connect failure";
const PEER_PROBE_BYTES: usize = 1;
const CLOSED_PEER_BYTES_READ: usize = 0;

#[test]
fn pending_connect_times_out_and_closes_its_owned_descriptor() {
    let backend = PendingBackend::default();
    let error = cleanup_connect::connect_with(
        Path::new(FIXTURE_SOCKET_NAME),
        CLEANUP_CONNECT_TIMEOUT,
        &backend,
    )
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert_peer_closed(&backend);
}

#[test]
fn refused_connect_is_classified_without_leaking_the_socket() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FIXTURE_SOCKET_NAME);
    drop(UnixListener::bind(&path).unwrap());

    let native_error = cleanup_connect::connect(&path, CLEANUP_CONNECT_TIMEOUT).unwrap_err();
    assert_eq!(native_error.kind(), io::ErrorKind::ConnectionRefused);

    let backend = PendingBackend {
        refuse_start: true,
        ..PendingBackend::default()
    };
    let state_error =
        cleanup_connect::connect_with(&path, CLEANUP_CONNECT_TIMEOUT, &backend).unwrap_err();
    assert_eq!(state_error.kind(), io::ErrorKind::ConnectionRefused);
    assert_peer_closed(&backend);
}

#[test]
fn every_pending_connect_error_closes_its_owned_descriptor() {
    for failure in [
        PendingFailure::Wait,
        PendingFailure::SocketError,
        PendingFailure::RestoreBlocking,
    ] {
        let backend = PendingBackend {
            failure,
            ..PendingBackend::default()
        };
        cleanup_connect::connect_with(
            Path::new(FIXTURE_SOCKET_NAME),
            CLEANUP_CONNECT_TIMEOUT,
            &backend,
        )
        .unwrap_err();
        assert_peer_closed(&backend);
    }
}

#[derive(Default)]
struct PendingBackend {
    peer: RefCell<Option<UnixStream>>,
    refuse_start: bool,
    failure: PendingFailure,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum PendingFailure {
    #[default]
    None,
    Wait,
    SocketError,
    RestoreBlocking,
}

impl cleanup_connect::ConnectBackend for PendingBackend {
    fn open(&self) -> io::Result<cleanup_connect::ConnectSocket> {
        let (stream, peer) = UnixStream::pair()?;
        *self.peer.borrow_mut() = Some(peer);
        let descriptor: OwnedFd = stream.into();
        Ok(cleanup_connect::ConnectSocket {
            descriptor,
            original_status: FIXTURE_ORIGINAL_STATUS,
        })
    }

    fn start(
        &self,
        _socket: &cleanup_connect::ConnectSocket,
        _path: &Path,
    ) -> io::Result<cleanup_connect::ConnectStart> {
        if self.refuse_start {
            return Err(io::Error::from_raw_os_error(libc::ECONNREFUSED));
        }
        Ok(cleanup_connect::ConnectStart::Pending)
    }

    fn wait(
        &self,
        _socket: &cleanup_connect::ConnectSocket,
        timeout: Duration,
    ) -> io::Result<cleanup_connect::ConnectWait> {
        assert_eq!(timeout, CLEANUP_CONNECT_TIMEOUT);
        if self.failure == PendingFailure::Wait {
            return Err(fixture_error());
        }
        if self.failure != PendingFailure::None {
            return Ok(cleanup_connect::ConnectWait::Ready);
        }
        Ok(cleanup_connect::ConnectWait::TimedOut)
    }

    fn socket_error(&self, _socket: &cleanup_connect::ConnectSocket) -> io::Result<libc::c_int> {
        if self.failure == PendingFailure::SocketError {
            return Err(fixture_error());
        }
        Ok(libc::c_int::default())
    }

    fn restore_blocking(&self, _socket: &cleanup_connect::ConnectSocket) -> io::Result<()> {
        if self.failure == PendingFailure::RestoreBlocking {
            return Err(fixture_error());
        }
        Ok(())
    }
}

fn fixture_error() -> io::Error {
    io::Error::other(FIXTURE_FAILURE_DETAIL)
}

fn assert_peer_closed(backend: &PendingBackend) {
    let mut peer = backend.peer.borrow_mut().take().unwrap();
    let mut byte = [0_u8; PEER_PROBE_BYTES];
    assert_eq!(peer.read(&mut byte).unwrap(), CLOSED_PEER_BYTES_READ);
}
