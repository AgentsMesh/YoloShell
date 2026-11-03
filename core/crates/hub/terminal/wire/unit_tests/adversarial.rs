use serde_json::json;
use tokio::{
    io::{AsyncWriteExt, duplex},
    time::{Duration, Instant},
};
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_jsonrpc_protocol::{Answer, Call, JSONRPC_VERSION, TERMINAL_EVENT_METHOD};
use yolo_hub_jsonrpc_transport::{Framed, Incoming, TransportError};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::*;
use yolo_hub_terminal_wire::*;

const PIPE_BYTES: usize = 2048;
const TIMEOUT: Duration = Duration::from_secs(3);
const BINARY_TAG: u8 = 1;
const CANCELLATION_TIMEOUT: Duration = Duration::from_millis(10);
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
fn output(target: RuntimeTarget, length: usize) -> serde_json::Value {
    json!({"jsonrpc":"2.0","method":TERMINAL_EVENT_METHOD,"params":{"event":"output", "chunk":{"target":target,"sequence":1,"byte_length":length}}})
}

#[tokio::test]
async fn missing_misordered_oversized_and_mismatched_binary_frames_fail_closed() {
    let target = target();
    for (length, bytes) in [
        (1, vec![]),
        (1, vec![BINARY_TAG, 0, 0, 0, 2, b'x', b'y']),
        (1, b"{\"jsonrpc\":\"2.0\"}\n".to_vec()),
        (MAX_OUTPUT_CHUNK_BYTES + 1, vec![]),
    ] {
        let (left, right) = duplex(PIPE_BYTES);
        let mut sender = Framed::new(left);
        sender
            .send(&output(target, length), deadline())
            .await
            .unwrap();
        let mut raw = sender.into_stream().unwrap();
        raw.write_all(&bytes).await.unwrap();
        raw.shutdown().await.unwrap();
        let mut receiver = Framed::new(right);
        assert!(matches!(
            receive_event(&mut receiver, deadline()).await,
            Err(Error::Transport(_))
        ));
        assert!(!receiver.is_usable());
    }
}

#[tokio::test]
async fn an_output_followed_by_heartbeat_is_not_conflated_and_foreign_epochs_never_advance() {
    let target = target();
    let cursor = OutputCursor {
        target,
        after_sequence: 0,
    };
    let foreign = RuntimeTarget {
        runtime_epoch: RuntimeEpoch::new(),
        ..target
    };
    let output = TerminalEvent::Output {
        chunk: OutputChunk::new(OutputChunkFields {
            target: foreign,
            sequence: 1,
            data: b"\0\n\xff".to_vec(),
        })
        .unwrap(),
    };
    let heartbeat = TerminalEvent::Heartbeat { cursor };
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    send_event(&mut sender, &output, deadline()).await.unwrap();
    send_event(&mut sender, &heartbeat, deadline())
        .await
        .unwrap();
    let mut receiver = Framed::new(right);
    let Incoming::Message(received) = receive_event(&mut receiver, deadline()).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(received, output);
    assert_eq!(received.advance(cursor), Err(PublicError::TargetMismatch));
    assert_eq!(
        receive_event(&mut receiver, deadline()).await.unwrap(),
        Incoming::Message(heartbeat)
    );
}

#[tokio::test]
async fn initial_metadata_rejects_identity_encoding_and_budget_before_reading_any_blob() {
    let target = target();
    let call = Call::new(
        target.address.hub_id,
        Command::Attach {
            request: AttachRequest::new(target, None).unwrap(),
        },
    )
    .unwrap();
    let body = json!({"stream":STREAM_ENCODING,"attachment":{"target":target,"snapshot":{"target":target,"barrier_sequence":0,"size":TerminalSize::default(),"byte_length":1},"chunks":[],(field::NEXT_SEQUENCE):1,"resynchronized":true}});
    for defect in 0..4 {
        let mut result = body.clone();
        match defect {
            0 => result["stream"] = json!("unknown"),
            1 => {
                result["attachment"]["target"][super::RUNTIME_EPOCH_FIELD] =
                    json!(RuntimeEpoch::new())
            }
            2 => result["attachment"]["snapshot"]["byte_length"] = json!(MAX_SNAPSHOT_BYTES + 1),
            _ => result["attachment"]["snapshot"]["data"] = json!([1]),
        }
        let answer = Answer {
            jsonrpc: JSONRPC_VERSION.into(),
            id: Some(call.id.clone()),
            result: Some(result),
            error: None,
        };
        let (left, _right) = duplex(PIPE_BYTES);
        let mut channel = Framed::new(left);
        assert!(matches!(
            receive_attachment(&mut channel, answer, &call, deadline()).await,
            Err(Error::Validation(_))
        ));
    }
}

#[tokio::test]
async fn cancellation_during_a_blob_retires_the_stream() {
    let (left, right) = duplex(PIPE_BYTES);
    let mut sender = Framed::new(left);
    sender.send(&output(target(), 1), deadline()).await.unwrap();
    let mut receiver = Framed::new(right);
    assert!(
        tokio::time::timeout(
            CANCELLATION_TIMEOUT,
            receive_event(&mut receiver, deadline())
        )
        .await
        .is_err()
    );
    assert!(!receiver.is_usable());
    assert!(matches!(
        receiver.receive_binary(1, 1, deadline()).await,
        Err(TransportError::Poisoned)
    ));
}
