use super::super::constants::STATUS_PEER_CLOSED;
use super::super::*;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ptr;
use std::time::Duration;
use yolo_relay::RelayError;

const TEST_SESSION_ID: u128 = 17;
const TEST_COLUMNS: u16 = 83;
const TEST_ROWS: u16 = 31;
const RUN_TIMEOUT: Duration = Duration::from_secs(2);
const HOLDER_ID: &str = "runtime-holder";
const HOST_EXECUTABLE_FILE: &str = "yolo-host-test";
const BROKER_SOCKET_FILE: &str = "broker.sock";
const EXPLICIT_SOCKET_FILE: &str = "explicit.sock";
const RESTORED_TERMINAL_FLAGS: libc::tcflag_t =
    libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN;

fn cli(home: Option<PathBuf>, socket: Option<PathBuf>, broker: Option<PathBuf>) -> Cli {
    Cli {
        home,
        socket,
        broker_socket: broker,
        terminal_id: uuid::Uuid::from_u128(TEST_SESSION_ID),
        holder_id: HOLDER_ID.into(),
        host_executable: None,
        take_control: false,
        status_fd: None,
        after_seq: INITIAL_OUTPUT_SEQUENCE,
    }
}

fn open_pty() -> (OwnedFd, OwnedFd) {
    let mut master = -1;
    let mut slave = -1;
    let mut window = libc::winsize {
        ws_row: TEST_ROWS,
        ws_col: TEST_COLUMNS,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: openpty initializes both descriptors and retains no pointers.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut window,
        )
    };
    assert_eq!(result, 0);
    // SAFETY: openpty returned two newly owned descriptors.
    unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
}

fn stop_resize_watcher() {
    // SAFETY: SIGWINCH is process-local and Tokio owns its installed handler.
    assert_eq!(unsafe { libc::raise(libc::SIGWINCH) }, 0);
}

fn terminal_flags(fd: libc::c_int) -> libc::tcflag_t {
    let mut value = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: the test supplies an open PTY and tcgetattr initializes value.
    assert_eq!(unsafe { libc::tcgetattr(fd, value.as_mut_ptr()) }, 0);
    // SAFETY: tcgetattr succeeded.
    unsafe { value.assume_init() }.c_lflag
}

#[test]
fn relay_runtime_selects_broker_explicit_and_derived_sockets() {
    let root = tempfile::tempdir().unwrap();
    let broker_socket = root.path().join(BROKER_SOCKET_FILE);
    let broker = relay_runtime(&cli(None, None, Some(broker_socket.clone()))).unwrap();
    assert_eq!(broker.client().socket_path(), broker_socket);

    let explicit_socket = root.path().join(EXPLICIT_SOCKET_FILE);
    let mut explicit = cli(
        Some(root.path().to_owned()),
        Some(explicit_socket.clone()),
        None,
    );
    explicit.host_executable = Some(root.path().join(HOST_EXECUTABLE_FILE));
    assert_eq!(
        relay_runtime(&explicit).unwrap().client().socket_path(),
        explicit_socket
    );

    let expected = Paths::new(root.path()).host_socket().unwrap();
    explicit.socket = None;
    assert_eq!(
        relay_runtime(&explicit).unwrap().client().socket_path(),
        expected
    );
}

#[tokio::test]
async fn cancelled_run_initializes_and_restores_the_terminal_path() {
    let root = tempfile::tempdir().unwrap();
    let (_master, slave) = open_pty();
    let original_flags = terminal_flags(slave.as_raw_fd());
    let cancellation = RelayCancellation::default();
    cancellation.cancel();
    let result = tokio::time::timeout(
        RUN_TIMEOUT,
        run(
            cli(None, None, Some(root.path().join(BROKER_SOCKET_FILE))),
            slave.as_raw_fd(),
            cancellation,
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(matches!(result.downcast_ref(), Some(RelayError::Cancelled)));
    assert_eq!(
        terminal_flags(slave.as_raw_fd()) & RESTORED_TERMINAL_FLAGS,
        original_flags & RESTORED_TERMINAL_FLAGS
    );
    stop_resize_watcher();
}

#[tokio::test]
async fn status_peer_failure_cancels_an_in_flight_broker_wait() {
    let root = tempfile::tempdir().unwrap();
    let (peer, transferred) = std::os::unix::net::UnixStream::pair().unwrap();
    let (_master, slave) = open_pty();
    let options = cli(None, None, Some(root.path().join(BROKER_SOCKET_FILE)));
    let reporter =
        StatusReporter::from_inherited_fd(Some(transferred.into_raw_fd())).unwrap();
    drop(peer);
    let error = tokio::time::timeout(
        RUN_TIMEOUT,
        run_with_reporter(
            options,
            slave.as_raw_fd(),
            RelayCancellation::default(),
            reporter,
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains(STATUS_PEER_CLOSED));
    stop_resize_watcher();
}
