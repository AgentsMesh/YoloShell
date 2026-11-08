use super::support::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_hub_dispatch::{Dispatcher, EventFuture, Operation, OperationFuture, TerminalSource};
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply, Request};
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, FIRST_OUTPUT_SEQUENCE, TerminalSize,
    TerminalSnapshot, TerminalSnapshotFields,
};

const SMALL_PIPE_BYTES: usize = 1;
const STREAM_PIPE_BYTES: usize = 1024;
const PROBE_BYTE: u8 = 1;

#[tokio::test]
async fn revocation_cancels_a_reply_blocked_on_full_output_without_a_client_read() {
    let fixture = Fixture::start().await;
    let config = fixture.registry.current().unwrap();
    let admission = fixture
        .registry
        .admit_verified_key(&config.clients[0].identity.public_key, now_ms())
        .unwrap();
    let (client, server) = tokio::io::duplex(SMALL_PIPE_BYTES);
    let task = tokio::spawn(yolo_hub_server::serve_admitted(
        server,
        fixture.backend.clone(),
        fixture.state.clone(),
        admission,
    ));
    let request =
        yolo_hub_jsonrpc_protocol::Call::new(fixture.state.hub_id(), Command::Hello {}).unwrap();
    let mut framed = Framed::new(client);
    framed.send(&request, deadline()).await.unwrap();
    let mut client = framed.into_stream().unwrap();
    // A prefix byte proves response production began; leave the remaining
    // response blocked behind the one-byte pipe and revoke at that boundary.
    tokio::time::timeout_at(deadline(), client.read_u8())
        .await
        .unwrap()
        .unwrap();
    fixture.registry.close();
    assert!(
        tokio::time::timeout_at(deadline(), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(client.write_all(&[PROBE_BYTE]).await.is_err());
}

struct AttachBackend {
    hub: HubId,
    initial: AttachResult,
    dropped: Arc<AtomicBool>,
}
struct QuietSource {
    initial: AttachResult,
    dropped: Arc<AtomicBool>,
}
impl Drop for QuietSource {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}
impl TerminalSource for QuietSource {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(std::future::pending())
    }
}
impl Dispatcher for AttachBackend {
    fn hub_id(&self) -> HubId {
        self.hub
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            Ok(match request.fields().command {
                Command::Hello {} => Operation::Reply(Reply::Hello {
                    hello: Hello {
                        hub_id: self.hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![
                            yolo_hub_rpc_protocol::Method::Hello,
                            yolo_hub_rpc_protocol::Method::Attach,
                        ],
                    },
                }),
                Command::Attach { .. } => Operation::Attached(Box::new(QuietSource {
                    initial: self.initial.clone(),
                    dropped: self.dropped.clone(),
                })),
                _ => panic!("unexpected request"),
            })
        })
    }
}

#[tokio::test]
async fn revoke_drops_an_idle_terminal_source_even_with_heartbeat_forwarding() {
    let fixture = Fixture::start().await;
    let mut config = fixture.registry.current().unwrap();
    let previous = config.revision;
    config.revision += 1;
    config.clients[0].origins[0].rights = [yolo_hub_routing::Right::TerminalRead]
        .into_iter()
        .collect();
    fixture.registry.replace(previous, config.clone()).unwrap();
    let admission = fixture
        .registry
        .admit_verified_key(&config.clients[0].identity.public_key, now_ms())
        .unwrap();
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: fixture.backend.allowed,
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let dropped = Arc::new(AtomicBool::new(false));
    let initial = AttachResult::new(AttachResultFields {
        target,
        snapshot: Some(
            TerminalSnapshot::new(TerminalSnapshotFields {
                target,
                barrier_sequence: FIRST_OUTPUT_SEQUENCE,
                size: TerminalSize::default(),
                data: vec![],
            })
            .unwrap(),
        ),
        chunks: vec![],
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE + 1,
        resynchronized: true,
    })
    .unwrap();
    let backend = Arc::new(AttachBackend {
        hub: fixture.state.hub_id(),
        initial,
        dropped: dropped.clone(),
    });
    let (client, server) = tokio::io::duplex(STREAM_PIPE_BYTES);
    let task = tokio::spawn(yolo_hub_server::serve_admitted(
        server,
        backend,
        fixture.state.clone(),
        admission,
    ));
    let mut client = yolo_hub_client::Client::open(client, fixture.state.hub_id(), deadline())
        .await
        .unwrap();
    client
        .call(
            fixture.backend.allowed,
            Command::Attach {
                request: AttachRequest::new(target, None).unwrap(),
            },
            deadline(),
        )
        .await
        .unwrap();
    let mut channel = client.into_channel().unwrap();
    fixture.registry.close();
    assert!(
        tokio::time::timeout_at(deadline(), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(dropped.load(Ordering::SeqCst));
    // Drain any heartbeat already serialized before revocation; closure must
    // arrive within the same absolute deadline, not be accepted as a timeout.
    let deadline = deadline();
    while let Incoming::Message(_) = yolo_hub_terminal_wire::receive_event(&mut channel, deadline)
        .await
        .unwrap()
    {}
}
