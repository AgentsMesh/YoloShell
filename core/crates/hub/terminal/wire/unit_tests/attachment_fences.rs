use serde_json::json;
use tokio::{
    io::duplex,
    time::{Duration, Instant},
};
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::{Answer, Call, Id, JSONRPC_VERSION};
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_wire::*;

const PIPE_BYTES: usize = 64;
const TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test]
async fn untrusted_attachment_metadata_cannot_consume_or_allocate_any_binary_payload() {
    let target = RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    };
    let call = Call::new(
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
    let chunk = json!({"target":target,"sequence":1,"byte_length":1});
    let base = json!({"stream":STREAM_ENCODING,"attachment":{"target":target,"snapshot":null,"chunks":[chunk],(field::NEXT_SEQUENCE):2,"resynchronized":false}});
    for defect in 0..8 {
        let mut answer = Answer {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result: Some(base.clone()),
            error: None,
        };
        match defect {
            0 => answer.id = Some(Id::fresh()),
            1 => answer.jsonrpc.clear(),
            2 => answer.result = None,
            3 => {
                answer.result.as_mut().unwrap()["attachment"]["chunks"][0]["byte_length"] = json!(0)
            }
            4 => {
                answer.result.as_mut().unwrap()["attachment"]["chunks"][0]["byte_length"] =
                    json!(MAX_OUTPUT_CHUNK_BYTES + 1)
            }
            5 => {
                answer.result.as_mut().unwrap()["attachment"]["chunks"][0]["target"]
                    [super::RUNTIME_EPOCH_FIELD] = json!(RuntimeEpoch::new())
            }
            6 => {
                answer.result.as_mut().unwrap()["attachment"]["chunks"] =
                    json!(vec![chunk.clone(); MAX_ATTACH_CHUNKS + 1])
            }
            _ => {
                let large =
                    json!({"target":target,"sequence":1,"byte_length":MAX_OUTPUT_CHUNK_BYTES});
                answer.result.as_mut().unwrap()["attachment"]["chunks"] = json!(vec![
                    large;
                    MAX_ATTACH_OUTPUT_BYTES / MAX_OUTPUT_CHUNK_BYTES
                        + 1
                ]);
            }
        }
        let (left, _right) = duplex(PIPE_BYTES);
        let mut receiver = Framed::new(left);
        assert!(matches!(
            receive_attachment(&mut receiver, answer, &call, Instant::now() + TIMEOUT).await,
            Err(Error::Validation(_))
        ));
        assert!(!receiver.is_usable());
    }
}
