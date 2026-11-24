use super::*;
use std::io;
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use yolo_runtime::model::{HostStatus, PROTOCOL_VERSION};
use yolo_runtime::protocol::{ResponseEnvelope, method};
use yolo_runtime_rpc_testing::Peer;

const TEST_BOOT_ID: &str = "bridge-bootstrap-boot";
const TEST_HOST_PID: u32 = 73;
const TEST_CONTROLLER_GENERATION: u64 = 11;
const TEST_HOST_SOCKET_FILE: &str = "host.sock";
const TEST_EARLY_EXIT_STATUS: &str = "exit status: 23";
const TEST_WINNER_PUBLISH_TIMEOUT: Duration = Duration::from_secs(2);
const SPAWN_CAPTURED: &str = "host role invocation captured without starting a service";

#[tokio::test]
async fn losing_bootstrap_waits_for_the_winner_to_publish() {
    let expected = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let wait_socket = socket.clone();
    let client = Client::managed_local(expected.path(), &wait_socket).unwrap();
    let (observed, child_polled) = oneshot::channel();
    let expected_home = expected.path().to_path_buf();
    let waiter = tokio::spawn(async move {
        let mut child = TestChild::exited_notifying(observed);
        let deadline = tokio::time::Instant::now() + TEST_WINNER_PUBLISH_TIMEOUT;
        let result = wait_for_host(&client, &wait_socket, &mut child, deadline).await;
        (result, child.preserved)
    });

    child_polled.await.unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    let winner = tokio::spawn(serve_status(listener, expected_home));
    let (result, preserved) = waiter.await.unwrap();
    result.unwrap();
    assert!(!preserved);
    winner.await.unwrap();
}

#[tokio::test]
async fn solo_early_exit_preserves_the_child_status() {
    let expected = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let client = Client::managed_local(expected.path(), &socket).unwrap();
    let mut child = TestChild::exited();
    let error = wait_for_host(&client, &socket, &mut child, tokio::time::Instant::now())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("{HOST_EARLY_EXIT_CONTEXT}: {TEST_EARLY_EXIT_STATUS}")
    );
}

#[tokio::test]
async fn winning_bootstrap_preserves_its_own_child() {
    let expected = tempfile::tempdir().unwrap();
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(TEST_HOST_SOCKET_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(serve_status(listener, expected.path().to_path_buf()));
    let client = Client::managed_local(expected.path(), &socket).unwrap();
    let mut child = TestChild::running(TEST_HOST_PID);
    let deadline = tokio::time::Instant::now() + TEST_WINNER_PUBLISH_TIMEOUT;
    wait_for_host(&client, &socket, &mut child, deadline)
        .await
        .unwrap();
    assert!(child.preserved);
    server.await.unwrap();
}

struct TestChild {
    observed: Option<oneshot::Sender<()>>,
    process_id: Option<u32>,
    exit_status: Option<&'static str>,
    preserved: bool,
}

impl TestChild {
    fn exited_notifying(observed: oneshot::Sender<()>) -> Self {
        Self::new(Some(observed), None, Some(TEST_EARLY_EXIT_STATUS))
    }

    fn exited() -> Self {
        Self::new(None, None, Some(TEST_EARLY_EXIT_STATUS))
    }

    fn running(process_id: u32) -> Self {
        Self::new(None, Some(process_id), None)
    }

    fn new(
        observed: Option<oneshot::Sender<()>>,
        process_id: Option<u32>,
        exit_status: Option<&'static str>,
    ) -> Self {
        Self {
            observed,
            process_id,
            exit_status,
            preserved: false,
        }
    }
}

impl HostChildStatus for TestChild {
    fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    fn try_wait_status(&mut self) -> io::Result<Option<String>> {
        if let Some(observed) = self.observed.take() {
            let _ = observed.send(());
        }
        Ok(self.exit_status.map(str::to_owned))
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

#[tokio::test]
async fn bridge_launches_the_explicit_host_role_with_its_exact_home_and_socket() {
    let home = tempfile::tempdir().unwrap();
    let sockets = tempfile::tempdir().unwrap();
    let socket = sockets.path().join(TEST_HOST_SOCKET_FILE);
    let paths = Paths::new(home.path());
    let expected = yolo_launch_contract::ProcessRole::Host.prepend([
        constants::SERVE_COMMAND.into(),
        constants::DETACHED_SESSION_ARGUMENT.into(),
        constants::HOME_ARGUMENT.into(),
        paths.home().as_os_str().to_owned(),
        constants::SOCKET_ARGUMENT.into(),
        socket.as_os_str().to_owned(),
    ]);
    let result = ensure_host_running_until(
        &paths,
        &socket,
        tokio::time::Instant::now() + TEST_WINNER_PUBLISH_TIMEOUT,
        |executable, arguments| {
            assert_eq!(executable, std::env::current_exe().unwrap());
            assert_eq!(arguments, expected);
            Err::<TestChild, _>(anyhow::anyhow!(SPAWN_CAPTURED))
        },
        std::future::ready(()),
    )
    .await;
    assert_eq!(result.unwrap_err().to_string(), SPAWN_CAPTURED);
}
