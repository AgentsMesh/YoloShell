use super::*;
use crate::model::PROTOCOL_VERSION;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method};
use crate::worker::{AttachRequest, AttachResult, attachment_next_sequence};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::net::UnixListener;

const TEST_BOOT_ID: &str = "local-auth-boot";
const TEST_HOST_PID: u32 = 41;
const TEST_CONTROLLER_GENERATION: u64 = 7;
const TEST_INITIAL_SEQUENCE: u64 = 0;
const TEST_RUNTIME_EPOCH_VALUE: u128 = 1;
const INCOMPATIBLE_VERSION_OFFSET: u16 = 1;
const RELATIVE_HOME: &str = "relative-home";
const UNUSED_SOCKET: &str = "/tmp/yolo-unused-host.sock";

struct AuthServer {
    _directory: TempDir,
    socket: PathBuf,
    methods: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl AuthServer {
    async fn start(homes: Vec<PathBuf>) -> Self {
        Self::start_with_version(homes, PROTOCOL_VERSION).await
    }

    async fn start_with_version(homes: Vec<PathBuf>, protocol_version: u16) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("host.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&methods);
        let task = tokio::spawn(async move {
            let mut homes = VecDeque::from(homes);
            while let Some(home) = homes.pop_front() {
                let (stream, _) = listener.accept().await.unwrap();
                let mut stream = crate::rpc_test_support::Peer::new(stream);
                while let Ok(request) = stream.request().await {
                    recorded.lock().unwrap().push(request.method.clone());
                    let response = response(&request, &home, protocol_version);
                    if stream.respond(&response).await.is_err() {
                        break;
                    }
                }
            }
        });
        Self {
            _directory: directory,
            socket,
            methods,
            task,
        }
    }

    async fn finish(self) -> Vec<String> {
        self.task.await.unwrap();
        self.methods.lock().unwrap().clone()
    }
}

fn response(request: &RequestEnvelope, home: &Path, protocol_version: u16) -> ResponseEnvelope {
    if request.method == method::HOST_STATUS {
        return ResponseEnvelope::success(request, &status(home, protocol_version)).unwrap();
    }
    if request.method == method::ATTACH_SESSION {
        let attachment: AttachRequest = request.decode_payload().unwrap();
        return ResponseEnvelope::success(request, &empty_attachment(attachment.after_seq))
            .unwrap();
    }
    ResponseEnvelope::success(request, &()).unwrap()
}

fn status(home: &Path, protocol_version: u16) -> HostStatus {
    HostStatus {
        protocol_version,
        boot_id: TEST_BOOT_ID.into(),
        host_pid: TEST_HOST_PID,
        controller_generation: TEST_CONTROLLER_GENERATION,
        home: home.to_path_buf(),
        capabilities: Vec::new(),
    }
}

fn empty_attachment(after_sequence: u64) -> AttachResult {
    AttachResult {
        runtime_epoch: Uuid::from_u128(TEST_RUNTIME_EPOCH_VALUE),
        snapshot: None,
        chunks: Vec::new(),
        next_sequence: attachment_next_sequence(after_sequence).unwrap(),
        resynchronized: false,
    }
}

#[test]
fn managed_local_rejects_an_unsafe_expected_home() {
    let error = Client::managed_local(RELATIVE_HOME, UNUSED_SOCKET).unwrap_err();
    assert!(matches!(error, ClientError::InvalidLocalHome { .. }));
}

#[tokio::test]
async fn mismatched_home_sends_no_mutating_request() {
    let expected = tempfile::tempdir().unwrap();
    let reported = tempfile::tempdir().unwrap();
    let server = AuthServer::start(vec![reported.path().to_path_buf()]).await;
    let client = Client::managed_local(expected.path(), &server.socket).unwrap();
    let error = client.discard(Uuid::new_v4(), true).await.unwrap_err();
    assert!(matches!(error, ClientError::HostHomeMismatch { .. }));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn invalid_reported_home_sends_no_followup_request() {
    let expected = tempfile::tempdir().unwrap();
    let server = AuthServer::start(vec![PathBuf::from(RELATIVE_HOME)]).await;
    let client = Client::managed_local(expected.path(), &server.socket).unwrap();
    let error = client.list_sessions(false).await.unwrap_err();
    assert!(matches!(error, ClientError::InvalidHostHome { .. }));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn prior_protocol_blocks_runtime_epoch_fenced_discard() {
    let expected = tempfile::tempdir().unwrap();
    let incompatible = PROTOCOL_VERSION
        .checked_sub(INCOMPATIBLE_VERSION_OFFSET)
        .unwrap();
    let server =
        AuthServer::start_with_version(vec![expected.path().to_path_buf()], incompatible).await;
    let client = Client::managed_local(expected.path(), &server.socket).unwrap();
    let error = client
        .discard_if_runtime_epoch(Uuid::new_v4(), true, Uuid::new_v4())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::HostProtocolMismatch { .. }));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn verified_status_reuses_the_connection_handshake() {
    let expected = tempfile::tempdir().unwrap();
    let server = AuthServer::start(vec![expected.path().to_path_buf()]).await;
    let client = Client::managed_local(expected.path(), &server.socket).unwrap();
    assert_eq!(client.status().await.unwrap().host_pid, TEST_HOST_PID);
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn every_attachment_reauthenticates_before_routing() {
    let expected = tempfile::tempdir().unwrap();
    let replacement = tempfile::tempdir().unwrap();
    let server = AuthServer::start(vec![
        expected.path().to_path_buf(),
        replacement.path().to_path_buf(),
    ])
    .await;
    let client = Client::managed_local(expected.path(), &server.socket).unwrap();
    let first = client
        .attach(Uuid::new_v4(), TEST_INITIAL_SEQUENCE)
        .await
        .unwrap();
    assert_eq!(
        first.initial.next_sequence,
        attachment_next_sequence(TEST_INITIAL_SEQUENCE).unwrap()
    );
    drop(first);
    let error = client
        .attach(Uuid::new_v4(), TEST_INITIAL_SEQUENCE)
        .await
        .err()
        .unwrap();
    assert!(matches!(error, ClientError::HostHomeMismatch { .. }));
    assert_eq!(
        server.finish().await,
        vec![
            method::HOST_STATUS,
            method::ATTACH_SESSION,
            method::HOST_STATUS,
        ]
    );
}
