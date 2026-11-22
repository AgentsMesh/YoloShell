use std::time::Duration;
use tokio::{io::DuplexStream, time::Instant};
use yolo_hub_client::Client;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Reply, SUPPORTED_METHODS};
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_client::Attachment;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const DUPLEX_BYTES: usize = 4096;
pub const OUTPUT: &[u8] = b"public terminal output";
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    }
}

pub async fn connected() -> (Attachment<DuplexStream>, Framed<DuplexStream>) {
    let target = target();
    let (client, peer) = tokio::io::duplex(DUPLEX_BYTES);
    let mut peer = Framed::new(peer);
    let connect = async {
        let client = Client::open(client, target.address.hub_id, deadline())
            .await
            .unwrap();
        Attachment::open(
            client,
            AttachRequest::new(target, None).unwrap(),
            deadline(),
        )
        .await
        .unwrap()
    };
    let serve = async {
        let Incoming::Message(request) = peer.receive::<Request>(deadline()).await.unwrap() else {
            panic!()
        };
        peer.send(
            &Response::new(
                &request,
                Reply::Hello {
                    hello: Hello {
                        hub_id: target.address.hub_id,
                        version: HUB_PROTOCOL_VERSION,
                        methods: SUPPORTED_METHODS.to_vec(),
                    },
                },
            )
            .unwrap(),
            deadline(),
        )
        .await
        .unwrap();
        let Incoming::Message(request) = peer.receive::<Request>(deadline()).await.unwrap() else {
            panic!()
        };
        let initial = AttachResult::new(AttachResultFields {
            target,
            snapshot: Some(
                TerminalSnapshot::new(TerminalSnapshotFields {
                    target,
                    barrier_sequence: 0,
                    size: TerminalSize::default(),
                    data: Vec::new(),
                })
                .unwrap(),
            ),
            chunks: Vec::new(),
            history: None,
            next_sequence: FIRST_OUTPUT_SEQUENCE,
            resynchronized: true,
        })
        .unwrap();
        yolo_hub_terminal_wire::send_attachment(&mut peer, &request, &initial, deadline())
            .await
            .unwrap();
    };
    let (attachment, ()) = tokio::join!(connect, serve);
    (attachment, peer)
}

pub fn output(cursor: OutputCursor) -> TerminalEvent {
    TerminalEvent::Output {
        chunk: OutputChunk::new(OutputChunkFields {
            target: cursor.target,
            sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE,
            data: OUTPUT.to_vec(),
        })
        .unwrap(),
    }
}
