use super::{Output, capture::Capture, deadline};
use tokio::{net::UnixListener, time::timeout_at};
use uuid::Uuid;
use yolo_runtime::{
    model::OutputChunk,
    protocol::{EventEnvelope, ResponseEnvelope, event, method},
    worker::AttachResult,
};

pub const FIRST_SEQUENCE: u64 = 1;
pub const BURST_CHUNKS: u64 = 128;
pub const BURST_BYTES: usize = 4_096;
pub const OUTPUT_BYTE: u8 = b'x';
pub const SCREEN_ROWS: u16 = 4;
pub const SCREEN_COLUMNS: u16 = 40;
pub const SCROLLBACK: usize = 0;
pub const MARKER: &[u8] = b"\x1b[2J\x1b[Hdriver-ready";
pub const READY_TEXT: &str = "driver-ready";
pub const SUCCESS_EXIT_CODE: i32 = 0;
const SOCKET_ROOT: &str = "/tmp";
const SOCKET_NAME: &str = "tui-driver-output.sock";
const INITIAL_CURSOR: u64 = 0;

pub(super) struct Fixture {
    pub peer: yolo_runtime_rpc_testing::Peer,
    pub session: Uuid,
    pub epoch: Uuid,
    pub output: Output,
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
            output: Output::start(attachment.unwrap()),
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

    pub async fn observed(&self, ready: impl Fn(&Capture) -> bool) {
        timeout_at(deadline(), async {
            loop {
                let changed = self.output.changed.notified();
                if ready(&self.output.capture.lock().unwrap()) {
                    return;
                }
                changed.await;
            }
        })
        .await
        .unwrap();
    }
}
