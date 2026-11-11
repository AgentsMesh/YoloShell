use super::*;
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsRawFd, FromRawFd};

const INVALID_DESCRIPTOR: RawFd = -1;
const DEFAULT_SOCKET_PROTOCOL: libc::c_int = 0;
const LOOPBACK_ADDRESS: &str = "127.0.0.1:0";
const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const PRIVATE_DESCRIPTOR_MINIMUM: RawFd = 128;
const SYSTEM_SUCCESS: libc::c_int = 0;
static DESCRIPTOR_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn connected_socketpair_is_adopted_with_private_nonblocking_flags() {
    let _guard = DESCRIPTOR_TEST_LOCK.lock().unwrap();
    let (peer, transferred) = std::os::unix::net::UnixStream::pair().unwrap();
    let raw = private_raw(&transferred);
    drop(transferred);

    let stream = adopt(raw).unwrap();

    let descriptor_flags = fcntl_get(stream.as_raw_fd(), libc::F_GETFD).unwrap();
    let status_flags = fcntl_get(stream.as_raw_fd(), libc::F_GETFL).unwrap();
    assert_ne!(descriptor_flags & libc::FD_CLOEXEC, libc::c_int::default());
    assert_ne!(status_flags & libc::O_NONBLOCK, libc::c_int::default());
    drop(peer);
    drop(stream);
    assert_descriptor_closed(raw);
}

#[tokio::test]
async fn negative_closed_and_stdio_descriptors_are_rejected_without_adoption() {
    let _guard = DESCRIPTOR_TEST_LOCK.lock().unwrap();
    for raw in [
        INVALID_DESCRIPTOR,
        libc::STDIN_FILENO,
        libc::STDOUT_FILENO,
        libc::STDERR_FILENO,
    ] {
        let error = adopt(raw).unwrap_err();
        assert!(error.to_string().contains(STATUS_FD_RESERVED));
    }
    let pipe = pipe();
    let raw = private_raw(&pipe.0);
    drop(pipe);
    assert_eq!(unsafe { libc::close(raw) }, SYSTEM_SUCCESS);
    let error = adopt(raw).unwrap_err();
    assert!(error.to_string().contains(STATUS_FD_OPEN_CONTEXT));
}

#[tokio::test]
async fn transferred_pipe_is_rejected_and_closed() {
    let _guard = DESCRIPTOR_TEST_LOCK.lock().unwrap();
    let (reader, writer) = pipe();
    let raw = private_raw(&reader);
    drop(reader);

    let error = adopt(raw).unwrap_err();

    assert!(error.to_string().contains(STATUS_FD_SOCKET_TYPE_CONTEXT));
    assert_descriptor_closed(raw);
    drop(writer);
}

#[tokio::test]
async fn unconnected_unix_stream_is_rejected_and_closed() {
    let _guard = DESCRIPTOR_TEST_LOCK.lock().unwrap();
    let raw = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, DEFAULT_SOCKET_PROTOCOL) };
    assert_ne!(raw, INVALID_DESCRIPTOR);
    // SAFETY: socket returned a newly owned descriptor.
    let socket = unsafe { std::os::fd::OwnedFd::from_raw_fd(raw) };
    let raw = private_raw(&socket);
    drop(socket);

    let error = adopt(raw).unwrap_err();

    assert!(error.to_string().contains(STATUS_FD_PEER_CONTEXT));
    assert_descriptor_closed(raw);
}

#[tokio::test]
async fn connected_tcp_stream_is_rejected_as_the_wrong_domain() {
    let _guard = DESCRIPTOR_TEST_LOCK.lock().unwrap();
    let listener = TcpListener::bind(LOOPBACK_ADDRESS).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (_server, _) = listener.accept().unwrap();
    let raw = private_raw(&client);
    drop(client);

    let error = adopt(raw).unwrap_err();

    assert!(error.to_string().contains(STATUS_FD_NOT_UNIX));
    assert_descriptor_closed(raw);
}

fn pipe() -> (std::os::fd::OwnedFd, std::os::fd::OwnedFd) {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, SYSTEM_SUCCESS);
    // SAFETY: pipe returned two newly owned descriptors.
    unsafe {
        (
            std::os::fd::OwnedFd::from_raw_fd(descriptors[PIPE_READER_INDEX]),
            std::os::fd::OwnedFd::from_raw_fd(descriptors[PIPE_WRITER_INDEX]),
        )
    }
}

fn assert_descriptor_closed(raw: RawFd) {
    let result = unsafe { libc::fcntl(raw, libc::F_GETFD) };
    assert_eq!(result, INVALID_DESCRIPTOR);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::EBADF));
}

fn private_raw(descriptor: &impl AsRawFd) -> RawFd {
    let raw = unsafe {
        libc::fcntl(
            descriptor.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            PRIVATE_DESCRIPTOR_MINIMUM,
        )
    };
    assert_ne!(raw, INVALID_DESCRIPTOR);
    raw
}
