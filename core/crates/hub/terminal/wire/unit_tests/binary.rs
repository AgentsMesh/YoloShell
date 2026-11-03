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

const TIMEOUT: Duration = Duration::from_secs(3);
const PIPE_BYTES: usize = 1024;
fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    }
}

#[tokio::test]
async fn snapshot_then_arbitrary_binary_output_and_exit_preserve_exact_bytes() {
    let target = target();
    let request = Call::new(
        target.address.hub_id,
        Command::Attach {
            request: AttachRequest::new(target, None).unwrap(),
        },
    )
    .unwrap();
    let bytes = (0..=u8::MAX).cycle().take(4096).collect::<Vec<_>>();
    let initial = AttachResult::new(AttachResultFields {
        target,
        snapshot: Some(
            TerminalSnapshot::new(TerminalSnapshotFields {
                target,
                barrier_sequence: 0,
                size: TerminalSize::default(),
                data: bytes.clone(),
            })
            .unwrap(),
        ),
        chunks: vec![],
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    let event = TerminalEvent::Output {
        chunk: OutputChunk::new(OutputChunkFields {
            target,
            sequence: FIRST_OUTPUT_SEQUENCE,
            data: bytes,
        })
        .unwrap(),
    };
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    let mut receiver = Framed::new(right);
    let send = async {
        send_attachment(&mut sender, &request, &initial, deadline())
            .await
            .unwrap();
        send_event(&mut sender, &event, deadline()).await.unwrap();
    };
    let receive = async {
        let Incoming::Message(answer) = receiver.receive::<Answer>(deadline()).await.unwrap()
        else {
            panic!()
        };
        let value = answer.result.as_ref().unwrap();
        assert_eq!(value["stream"], STREAM_ENCODING);
        assert!(value["attachment"]["snapshot"].get("data").is_none());
        let received = receive_attachment(&mut receiver, answer, &request, deadline())
            .await
            .unwrap();
        assert_eq!(
            received,
            Reply::Attached {
                attachment: initial.clone()
            }
        );
        assert_eq!(
            receive_event(&mut receiver, deadline()).await.unwrap(),
            Incoming::Message(event.clone())
        );
    };
    tokio::join!(send, receive);
}
