use super::{Probe, deadline};
use tokio::{net::UnixListener, time::timeout_at};
use uuid::Uuid;
use yolo_runtime::{
    model::OutputChunk,
    protocol::{EventEnvelope, ProtocolError, ResponseEnvelope, event, method},
    worker::AttachResult,
};

pub const BURST_CHUNKS: u64 = 64;
pub const FIRST_SEQUENCE: u64 = 1;
pub const OUTPUT_BYTE: &[u8] = b"x";
const SOCKET_ROOT: &str = "/tmp";
const SOCKET_NAME: &str = "clipboard-probe.sock";
const INITIAL_CURSOR: u64 = 0;

pub struct Fixture {
    pub peer: yolo_runtime_rpc_testing::Peer,
    pub session: Uuid,
    pub epoch: Uuid,
    pub probe: Probe,
    _root: tempfile::TempDir,
}

impl Fixture {
    pub async fn start() -> Self {
        let root = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
        let socket = root.path().join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let session = Uuid::new_v4();
        let epoch = Uuid::new_v4();
        let client = yolo_client::Client::new(socket);
        let handshake = async {
            let (stream, _) = listener.accept().await.unwrap();
            let mut peer = yolo_runtime_rpc_testing::Peer::new(stream);
            let request = peer.request().await.unwrap();
            assert_eq!(request.method, method::ATTACH_SESSION);
            let response = ResponseEnvelope::success(
                &request,
                &AttachResult {
                    runtime_epoch: epoch,
                    snapshot: None,
                    chunks: Vec::new(),
                    next_sequence: FIRST_SEQUENCE,
                    resynchronized: false,
                },
            )
            .unwrap();
            peer.respond(&response).await.unwrap();
            peer
        };
        let (peer, attachment) = timeout_at(deadline(), async {
            tokio::join!(handshake, client.attach(session, INITIAL_CURSOR))
        })
        .await
        .unwrap();
        Self {
            peer,
            session,
            epoch,
            probe: Probe::start(attachment.unwrap()),
            _root: root,
        }
    }

    pub async fn output(&mut self, sequence: u64, data: Vec<u8>) {
        let chunk = OutputChunk {
            runtime_epoch: self.epoch,
            sequence,
            data,
        };
        let message = EventEnvelope::new(
            event::TERMINAL_OUTPUT,
            Some(self.session.to_string()),
            Some(self.epoch.to_string()),
            &chunk,
        )
        .unwrap();
        timeout_at(deadline(), self.peer.event(&message))
            .await
            .unwrap()
            .unwrap();
    }

    /// Observation is non-consuming: no assert_commands call drains output.
    pub async fn observed(&self, ready: impl Fn(&super::capture::Capture) -> bool) {
        timeout_at(deadline(), async {
            loop {
                let changed = self.probe.changed.notified();
                if ready(&self.probe.capture.lock().unwrap()) {
                    return;
                }
                changed.await;
            }
        })
        .await
        .unwrap();
    }

    pub async fn closed(&mut self) {
        assert!(matches!(
            timeout_at(deadline(), self.peer.request()).await.unwrap(),
            Err(ProtocolError::EndOfStream)
        ));
    }
}
