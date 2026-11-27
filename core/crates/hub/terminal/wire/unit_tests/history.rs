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

#[tokio::test]
async fn history_blob_is_one_bounded_frame_after_orderly_metadata() {
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
            request: AttachRequest::history(target, None).unwrap(),
        },
    )
    .unwrap();
    let data = (0..=u8::MAX).cycle().take(4096).collect::<Vec<_>>();
    let initial = AttachResult::new(AttachResultFields {
        target,
        snapshot: None,
        chunks: Vec::new(),
        history: Some(
            TerminalHistory::new(TerminalHistoryFields {
                target,
                barrier_sequence: 2,
                size: TerminalSize::default(),
                data,
            })
            .unwrap(),
        ),
        next_sequence: 3,
        resynchronized: false,
    })
    .unwrap();
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    let mut receiver = Framed::new(right);
    let send = async {
        send_attachment(&mut sender, &request, &initial, Instant::now() + TIMEOUT)
            .await
            .unwrap();
    };
    let receive = async {
        let Incoming::Message(answer) = receiver
            .receive::<Answer>(Instant::now() + TIMEOUT)
            .await
            .unwrap()
        else {
            panic!()
        };
        let value = answer.result.as_ref().unwrap();
        assert_eq!(value["stream"], STREAM_ENCODING);
        assert!(value["attachment"]["history"].get("data").is_none());
        assert_eq!(
            receive_attachment(&mut receiver, answer, &request, Instant::now() + TIMEOUT)
                .await
                .unwrap(),
            Reply::Attached {
                attachment: initial.clone()
            }
        );
    };
    tokio::join!(send, receive);
}
