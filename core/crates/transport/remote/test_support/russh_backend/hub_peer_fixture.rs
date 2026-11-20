use super::{
    constants::*,
    fixture::{MemoryConnection, keys},
    memory_fs::MemoryFilesystem,
};
use crate::remote::*;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use std::path::PathBuf;
use tokio::time::Instant;
#[path = "hub_peer_evidence.rs"]
mod evidence;
pub use evidence::diagnostic as peer_diagnostic;
pub use evidence::{
    Phase, assert_exact_peer_close, incoming_burst, observe_phase, wait_peer_close,
};

pub const PAYLOAD: &[u8] = b"inner-Hub\0opaque-binary\r\n";
pub const LARGE_PAYLOAD_BYTES: usize = 1_048_576;
pub const PAYLOAD_FILL: u8 = b'P';
pub const EXPECTED_HUB_PEER_CHANNELS: usize = 1;
const TARGET_SOCKET: &str = "hub.sock";

pub struct PeerTarget {
    _root: tempfile::TempDir,
    pub listener: tokio::net::UnixListener,
    pub path: PathBuf,
}

impl PeerTarget {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(TARGET_SOCKET);
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        Self {
            _root: root,
            listener,
            path,
        }
    }
}

pub struct PeerFixture {
    pub ssh: MemoryConnection,
    pub target: PeerTarget,
    pub request: HubPeerRequest,
}

impl PeerFixture {
    pub async fn new() -> Self {
        Self::with_timeout(NORMAL_OPERATION_TIMEOUT).await
    }
    pub async fn with_timeout(timeout: std::time::Duration) -> Self {
        let target = PeerTarget::new();
        let filesystem = MemoryFilesystem::default();
        filesystem.set_hub_peer_target(target.path.clone());
        let ssh = super::fixture::connect_memory_with_timeout(false, filesystem, timeout)
            .await
            .unwrap();
        let identity = identity();
        let request = HubPeerRequest::new(
            ssh.connection.info().connection_id,
            HubPeerEndpoint::new(target.path.clone()).unwrap(),
            identity,
        )
        .unwrap();
        Self {
            ssh,
            target,
            request,
        }
    }
}

fn identity() -> PeerIdentity {
    let (key, _) = super::fixture::keys();
    let bytes = key.public_key().key_data().ed25519().unwrap().0;
    PeerIdentity::new(HubId::new(), bytes).unwrap()
}

pub fn command() -> AuthorizedCommand {
    CommandPolicy::new([NORMAL_PROGRAM.into()])
        .unwrap()
        .authorize(CommandRequest::new(NORMAL_PROGRAM, Vec::new()))
        .unwrap()
}

pub fn deadline() -> Instant {
    Instant::now() + TEST_INACTIVITY_TIMEOUT
}

pub async fn assert_connection_healthy(fixture: &MemoryConnection) {
    assert_connection_healthy_at(fixture, Instant::now()).await;
}

pub async fn assert_connection_healthy_at(fixture: &MemoryConnection, started: Instant) {
    assert!(
        !fixture.connection.session.is_closed(),
        "SSH closed; {}",
        evidence::diagnostic(fixture, started, Phase::SiblingExec)
    );
    let command = command();
    let output = observe_phase(
        fixture,
        started,
        Phase::SiblingExec,
        fixture.connection.execute(&command),
    )
    .await;
    let diagnostic = evidence::diagnostic(fixture, started, Phase::SiblingExec);
    assert_eq!(output.value.stdout, COMMAND_STDOUT, "{diagnostic}");
    assert_eq!(
        output.identity.connection_id,
        fixture.connection.info().connection_id,
        "{diagnostic}"
    );
}
