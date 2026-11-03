use tokio::{
    io::duplex,
    time::{Duration, Instant},
};
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_wire::*;

const PIPE_BYTES: usize = 2048;
const TIMEOUT: Duration = Duration::from_secs(3);
fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}

#[tokio::test]
async fn replay_chunks_and_all_non_output_notifications_keep_their_order() {
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let request = Call::new(
        target.address.hub_id,
        Command::Attach {
            request: AttachRequest::new(
                target,
                Some(OutputCursor {
                    target,
                    after_sequence: 0,
                }),
            )
            .unwrap(),
        },
    )
    .unwrap();
    let chunks = [b"\0".to_vec(), b"\n\xff".to_vec()]
        .into_iter()
        .enumerate()
        .map(|(index, data)| {
            OutputChunk::new(OutputChunkFields {
                target,
                sequence: index as u64 + FIRST_OUTPUT_SEQUENCE,
                data,
            })
            .unwrap()
        })
        .collect();
    let initial = AttachResult::new(AttachResultFields {
        target,
        snapshot: None,
        chunks,
        history: None,
        next_sequence: 3,
        resynchronized: false,
    })
    .unwrap();
    let cursor = OutputCursor {
        target,
        after_sequence: 2,
    };
    let events = [
        TerminalEvent::Heartbeat { cursor },
        TerminalEvent::Exited {
            cursor,
            code: Some(0),
            signal: None,
        },
        TerminalEvent::ResyncRequired { target },
        TerminalEvent::Unreachable { target },
    ];
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    let mut receiver = Framed::new(right);
    let send = async {
        send_attachment(&mut sender, &request, &initial, deadline())
            .await
            .unwrap();
        for event in &events {
            send_event(&mut sender, event, deadline()).await.unwrap();
        }
        sender.close(deadline()).await.unwrap();
    };
    let receive = async {
        let Incoming::Message(answer) = receiver.receive::<Answer>(deadline()).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(
            receive_attachment(&mut receiver, answer, &request, deadline())
                .await
                .unwrap(),
            Reply::Attached {
                attachment: initial.clone()
            }
        );
        for event in &events {
            assert_eq!(
                receive_event(&mut receiver, deadline()).await.unwrap(),
                Incoming::Message(event.clone())
            );
        }
        assert_eq!(
            receive_event(&mut receiver, deadline()).await.unwrap(),
            Incoming::Eof
        );
    };
    tokio::join!(send, receive);
}

#[tokio::test]
async fn notification_version_method_unknown_fields_and_legacy_byte_arrays_are_rejected() {
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let base = serde_json::json!({"jsonrpc":"2.0","method":yolo_hub_jsonrpc_protocol::TERMINAL_EVENT_METHOD,"params":{"event":"heartbeat","cursor":{"target":target,(field::AFTER_SEQUENCE):0}}});
    for (field, value) in [
        ("jsonrpc", serde_json::json!("1.0")),
        ("method", serde_json::json!("input")),
        ("id", serde_json::json!("must-be-notification")),
        (
            "params",
            serde_json::json!({"event":"output","chunk":{"target":target,"sequence":1,"data":[1]}}),
        ),
    ] {
        let mut malformed = base.clone();
        malformed[field] = value;
        let (left, right) = duplex(PIPE_BYTES);
        Framed::new(left)
            .send(&malformed, deadline())
            .await
            .unwrap();
        let mut receiver = Framed::new(right);
        assert!(receive_event(&mut receiver, deadline()).await.is_err());
        assert!(!receiver.is_usable());
    }
}
