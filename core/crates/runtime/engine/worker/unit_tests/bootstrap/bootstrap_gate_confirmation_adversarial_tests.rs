use super::*;
use crate::local_unix::LocalUnixStream;
use crate::process::{BootstrapExecTransitionAuthority, ProcessIdentity};
use crate::protocol::{FRAME_LENGTH_BYTES, encode, to_value};
use std::process::{Child, Command};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use yolo_capnp_value::Value;

const EXEC_FAILURE_DETAIL: &str = "authenticated exec failure";
const INVALID_CAPNP_BYTE: u8 = 0xc1;
const OPEN_STREAM_TIMEOUT: Duration = Duration::from_millis(50);
const SLEEP_ARGUMENT: &str = "30";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SINGLE_BYTE_COUNT: usize = std::mem::size_of::<u8>();
const UNEXPECTED_CONFIRMATION_DETAIL: &str =
    "uncertain bootstrap report produced an exec confirmation";
const UNKNOWN_FIELD: &str = "unexpected";

struct ObservedChild(Child);

#[tokio::test]
async fn complete_failure_frame_is_the_only_explicit_failure_authority() {
    let confirmation = confirm_bytes(&encode(&failure_frame()).unwrap())
        .await
        .unwrap();
    let BootstrapExecConfirmation::ExplicitFailure(failure) = confirmation else {
        panic!("{UNEXPECTED_CONFIRMATION_DETAIL}");
    };
    assert_eq!(failure.detail(), EXEC_FAILURE_DETAIL);
}

#[tokio::test]
async fn open_stream_timeout_fails_closed() {
    let (gate, _open_peer, _child) = released_gate();
    let result = gate
        .confirm_exec_before(
            BootstrapExecTransitionAuthority::for_test(),
            Instant::now() + OPEN_STREAM_TIMEOUT,
        )
        .await;
    assert_fail_closed(result);
}

#[tokio::test]
async fn every_partial_header_length_fails_closed() {
    let frame = encode(&failure_frame()).unwrap();
    for received in SINGLE_BYTE_COUNT..FRAME_LENGTH_BYTES {
        assert_fail_closed(confirm_bytes(&frame[..received]).await);
    }
}

#[tokio::test]
async fn truncated_payload_and_invalid_capnp_fail_closed() {
    let mut truncated = encode(&failure_frame()).unwrap();
    assert!(truncated.pop().is_some());
    assert_fail_closed(confirm_bytes(&truncated).await);
    assert_fail_closed(confirm_bytes(&raw_frame(&[INVALID_CAPNP_BYTE])).await);
}

#[tokio::test]
async fn unknown_and_duplicate_failure_fields_fail_closed() {
    let frame = to_value(&failure_frame()).unwrap();
    assert_fail_closed(confirm_value(with_unknown(frame.clone())).await);
    assert_fail_closed(confirm_value(with_duplicate(frame)).await);
}

async fn confirm_value(value: Value) -> anyhow::Result<BootstrapExecConfirmation> {
    confirm_bytes(&raw_frame(&yolo_capnp_codec::encode_value(&value).unwrap())).await
}

async fn confirm_bytes(bytes: &[u8]) -> anyhow::Result<BootstrapExecConfirmation> {
    let (gate, mut peer, _child) = released_gate();
    peer.write_all(bytes).await.unwrap();
    peer.shutdown().await.unwrap();
    gate.confirm_exec_before(
        BootstrapExecTransitionAuthority::for_test(),
        Instant::now() + super::super::BOOTSTRAP_GATE_TIMEOUT,
    )
    .await
}

fn released_gate() -> (ReleasedBootstrapGate, LocalUnixStream, ObservedChild) {
    let child = ObservedChild(crate::process::spawn_waitable_test_child(
        Command::new(SLEEP_EXECUTABLE).arg(SLEEP_ARGUMENT),
    ));
    let process = ProcessIdentity::capture(child.0.id()).unwrap();
    let observer =
        super::super::BootstrapExecObserver::register(super::super::BootstrapExecTarget {
            process,
            thread_id: process.pid,
        })
        .unwrap();
    let (stream, peer) = LocalUnixStream::pair().unwrap();
    (ReleasedBootstrapGate(stream, observer), peer, child)
}

fn failure_frame() -> BootstrapExecFailureFrame {
    BootstrapExecFailureFrame {
        detail: EXEC_FAILURE_DETAIL.to_owned(),
    }
}

fn with_unknown(mut value: Value) -> Value {
    map_mut(&mut value).push((key(UNKNOWN_FIELD), Value::Boolean(true)));
    value
}

fn with_duplicate(mut value: Value) -> Value {
    let duplicate = map_mut(&mut value).first().unwrap().clone();
    map_mut(&mut value).push(duplicate);
    value
}

fn raw_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = u32::try_from(payload.len()).unwrap().to_be_bytes().to_vec();
    frame.extend_from_slice(payload);
    frame
}

fn map_mut(value: &mut Value) -> &mut Vec<(Value, Value)> {
    let Value::Map(map) = value else {
        panic!("{UNEXPECTED_CONFIRMATION_DETAIL}");
    };
    map
}

fn key(value: &str) -> Value {
    Value::Text(value.into())
}

fn assert_fail_closed(result: anyhow::Result<BootstrapExecConfirmation>) {
    assert!(result.is_err(), "{UNEXPECTED_CONFIRMATION_DETAIL}");
}

impl Drop for ObservedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
