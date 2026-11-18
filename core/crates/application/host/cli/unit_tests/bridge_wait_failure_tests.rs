use super::*;
use std::io;
use tokio::net::UnixListener;
use yolo_runtime::model::{HostStatus, PROTOCOL_VERSION};
use yolo_runtime::protocol::{ResponseEnvelope, method};
use yolo_runtime_rpc_testing::Peer;

const TEST_BOOT_ID: &str = "bridge-wait-failure-boot";
const TEST_HOST_PID: u32 = 701;
const TEST_CHILD_PID: u32 = 702;
const TEST_CONTROLLER_GENERATION: u64 = 17;
const TEST_HOST_SOCKET_FILE: &str = "host.sock";
const TEST_POLL_FAILURE: &str = "intentional child poll failure";
const TEST_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const INITIAL_POLL_COUNT: usize = 0;
const SINGLE_POLL_COUNT: usize = 1;
const EXPIRED_WAIT_POLL_COUNT: usize = 2;
const VERSION_MISMATCH_INCREMENT: u16 = 1;

#[tokio::test]
async fn wait_propagates_authenticated_host_identity_failure() {
    let expected = tempfile::tempdir().unwrap();
    let reported = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(serve_status(listener, reported.path().to_path_buf()));
    let client = Client::managed_local(expected.path(), &socket).unwrap();
    let mut child = WaitChild::running();

    let error = wait_for_host(
        &client,
        &socket,
        &mut child,
        tokio::time::Instant::now() + TEST_WAIT_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ClientError>(),
        Some(ClientError::HostHomeMismatch { .. })
    ));
    assert!(!child.preserved);
    server.await.unwrap();
}

#[tokio::test]
async fn wait_contextualizes_child_poll_failure() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let client = Client::managed_local(home.path(), &socket).unwrap();
    let mut child = WaitChild::failing();

    let error = wait_for_host(
        &client,
        &socket,
        &mut child,
        tokio::time::Instant::now() + TEST_WAIT_TIMEOUT,
    )
    .await
    .unwrap_err();
    let diagnostic = format!("{error:#}");
    assert!(diagnostic.contains(HOST_STATUS_CONTEXT));
    assert!(diagnostic.contains(TEST_POLL_FAILURE));
}

#[tokio::test]
async fn deadline_without_child_exit_reports_not_ready() {
    let home = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let client = Client::managed_local(home.path(), &socket).unwrap();
    let mut child = WaitChild::running();

    let error = wait_for_host(&client, &socket, &mut child, tokio::time::Instant::now())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("{HOST_NOT_READY_CONTEXT}: {}", socket.display())
    );
    assert_eq!(child.polls, EXPIRED_WAIT_POLL_COUNT);
}

#[test]
fn transport_classifier_accepts_only_deadline_and_stream_failures() {
    for error in [
        ClientError::Deadline,
        ClientError::Protocol(ProtocolError::EndOfStream),
        ClientError::Protocol(ProtocolError::OutcomeUnknown),
        ClientError::Protocol(ProtocolError::Io(io::ErrorKind::BrokenPipe.into())),
    ] {
        assert!(is_transport_error(&error));
    }
    assert!(!is_transport_error(&ClientError::HostProtocolMismatch {
        expected: PROTOCOL_VERSION,
        reported: PROTOCOL_VERSION + VERSION_MISMATCH_INCREMENT,
    }));
}

struct WaitChild {
    fail: bool,
    polls: usize,
    preserved: bool,
}

impl WaitChild {
    fn running() -> Self {
        Self {
            fail: false,
            polls: INITIAL_POLL_COUNT,
            preserved: false,
        }
    }

    fn failing() -> Self {
        Self {
            fail: true,
            ..Self::running()
        }
    }
}

impl HostChildStatus for WaitChild {
    fn process_id(&self) -> Option<u32> {
        Some(TEST_CHILD_PID)
    }

    fn try_wait_status(&mut self) -> io::Result<Option<String>> {
        self.polls += SINGLE_POLL_COUNT;
        if self.fail {
            Err(io::Error::other(TEST_POLL_FAILURE))
        } else {
            Ok(None)
        }
    }

    fn preserve(&mut self) {
        self.preserved = true;
    }
}

async fn serve_status(listener: UnixListener, home: PathBuf) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut peer = Peer::new(stream);
    let request = peer.request().await.unwrap();
    assert_eq!(request.method, method::HOST_STATUS);
    let response = ResponseEnvelope::success(&request, &status(home)).unwrap();
    peer.respond(&response).await.unwrap();
    assert!(peer.request().await.is_err());
}

fn status(home: PathBuf) -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home,
        capabilities: Vec::new(),
    }
}
