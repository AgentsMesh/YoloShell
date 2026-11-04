use super::tests::test_spec;
use super::*;
#[cfg(target_os = "macos")]
use crate::process::ProcessIdentity;
use crate::protocol::{ProtocolError, encode, to_value};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWriteExt;
use yolo_capnp_value::Value;

const FRAME_BUFFER_CAPACITY: usize = 4_096;
const UNKNOWN_FIELD: &str = "unexpected";
const TEST_SOCKET_FILE: &str = "protocol-adversarial.sock";
const TEST_MARKER_FILE: &str = "protocol-adversarial.marker";
const MISMATCH_UUID: &str = "00000000-0000-0000-0000-000000000000";
const TRUNCATED_FRAME_BYTES: usize = 1;
const ACK_READER_HANDLE: u64 = 41;
const ACK_WRITER_HANDLE: u64 = 43;
const MAP_VALUE_EXPECTED_DETAIL: &str = "encoded protocol value must be a map";

#[tokio::test]
async fn hello_and_ack_reject_unknown_and_duplicate_fields_in_real_frames() {
    let directory = tempfile::tempdir().unwrap();
    let spec = test_spec(
        directory.path().join(TEST_SOCKET_FILE),
        &directory.path().join(TEST_MARKER_FILE),
    );
    let hello = to_value(&spec.hello().unwrap()).unwrap();
    let ack = to_value(&BootstrapCapabilityAck {
        reader_handle: ACK_READER_HANDLE,
        writer_handle: ACK_WRITER_HANDLE,
    })
    .unwrap();
    assert_rejected::<BootstrapHello>(with_unknown(hello.clone())).await;
    assert_rejected::<BootstrapHello>(with_duplicate(hello)).await;
    assert_rejected::<BootstrapCapabilityAck>(with_unknown(ack.clone())).await;
    assert_rejected::<BootstrapCapabilityAck>(with_duplicate(ack)).await;
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn every_hello_identity_component_is_checked_by_the_live_gate() {
    for field in [
        crate::protocol::field::TERMINAL_ID,
        crate::protocol::field::RUNTIME_EPOCH,
        crate::protocol::field::WORKER_ID,
        super::protocol::field::TOKEN,
    ] {
        assert_live_hello_rejected(field, key(MISMATCH_UUID), BOOTSTRAP_GATE_HELLO_CONTEXT).await;
    }
    assert_live_hello_rejected(
        super::protocol::field::EXEC_THREAD_ID,
        Value::from(u64::default()),
        BOOTSTRAP_EXEC_THREAD_CONTEXT,
    )
    .await;
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn swapped_ack_endpoints_survive_framing_but_fail_exact_validation() {
    let issuer = crate::process::MacosProcessCapabilityIssuer::create().unwrap();
    let identity = issuer.identity();
    let ack: BootstrapCapabilityAck = frame_roundtrip(&BootstrapCapabilityAck {
        reader_handle: identity.writer_handle(),
        writer_handle: identity.reader_handle(),
    })
    .await
    .unwrap();
    assert!(
        issuer
            .commit_acknowledgement(ack.reader_handle, ack.writer_handle)
            .is_err()
    );
}

#[tokio::test]
async fn truncated_hello_and_ack_frames_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let spec = test_spec(
        directory.path().join(TEST_SOCKET_FILE),
        &directory.path().join(TEST_MARKER_FILE),
    );
    assert_truncated::<BootstrapHello>(&spec.hello().unwrap()).await;
    assert_truncated::<BootstrapCapabilityAck>(&BootstrapCapabilityAck {
        reader_handle: ACK_READER_HANDLE,
        writer_handle: ACK_WRITER_HANDLE,
    })
    .await;
}

#[cfg(target_os = "macos")]
async fn assert_live_hello_rejected(field: &str, replacement: Value, expected_detail: &str) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(TEST_SOCKET_FILE);
    let marker = directory.path().join(TEST_MARKER_FILE);
    let listener = UnixListener::bind(&socket).unwrap();
    let spec = test_spec(socket.clone(), &marker);
    let expected = spec.expected(ProcessIdentity::capture(std::process::id()).unwrap());
    let hello = replace_field(
        to_value(&spec.hello().unwrap()).unwrap(),
        field,
        replacement,
    );
    let client = async move {
        let mut stream = UnixStream::connect(socket).await.unwrap();
        write_frame(&mut stream, &hello).await.unwrap();
    };
    let ((), result) = tokio::join!(client, accept_bootstrap_gate(&listener, &expected));
    assert!(result.err().unwrap().to_string().contains(expected_detail));
}

async fn assert_rejected<T: DeserializeOwned>(value: Value) {
    let payload = yolo_capnp_codec::encode_value(&value).unwrap();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let (mut writer, mut reader) = tokio::io::duplex(FRAME_BUFFER_CAPACITY);
    writer.write_all(&frame).await.unwrap();
    drop(writer);
    assert!(matches!(
        read_frame::<_, T>(&mut reader).await,
        Err(ProtocolError::Decode(_))
    ));
}

async fn assert_truncated<T: DeserializeOwned>(value: &impl Serialize) {
    let mut frame = encode(value).unwrap();
    frame.truncate(frame.len() - TRUNCATED_FRAME_BYTES);
    let (mut writer, mut reader) = tokio::io::duplex(FRAME_BUFFER_CAPACITY);
    writer.write_all(&frame).await.unwrap();
    drop(writer);
    assert!(matches!(
        read_frame::<_, T>(&mut reader).await,
        Err(ProtocolError::Io(error))
            if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));
}

async fn frame_roundtrip<S: Serialize, T: DeserializeOwned>(value: &S) -> Result<T, ProtocolError> {
    let (mut writer, mut reader) = tokio::io::duplex(FRAME_BUFFER_CAPACITY);
    write_frame(&mut writer, value).await.unwrap();
    drop(writer);
    read_frame(&mut reader).await
}

fn with_unknown(mut value: Value) -> Value {
    map_mut(&mut value).push((key(UNKNOWN_FIELD), Value::Boolean(true)));
    value
}

fn with_duplicate(mut value: Value) -> Value {
    let map = map_mut(&mut value);
    let duplicate = map.first().unwrap().clone();
    map.push(duplicate);
    value
}

fn replace_field(mut value: Value, field: &str, replacement: Value) -> Value {
    let entry = map_mut(&mut value)
        .iter_mut()
        .find(|(name, _)| name.as_str() == Some(field))
        .unwrap();
    entry.1 = replacement;
    value
}

fn key(value: &str) -> Value {
    Value::Text(value.into())
}

fn map_mut(value: &mut Value) -> &mut Vec<(Value, Value)> {
    match value {
        Value::Map(map) => map,
        _ => panic!("{MAP_VALUE_EXPECTED_DETAIL}"),
    }
}
