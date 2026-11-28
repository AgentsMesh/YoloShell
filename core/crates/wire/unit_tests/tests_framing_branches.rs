use super::*;
use serde::Serialize;
use serde::ser::Error as _;
use std::io::{Cursor, Read};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use yolo_capnp_value::Value;

const REQUEST_ID: &str = "framing-branch-request";
const REQUEST_METHOD: &str = "framing-branch-method";
const SERIALIZATION_FAILURE: &str = "intentional serialization failure";
const TRUNCATED_PAYLOAD_LENGTH: u32 = 3;
const TRUNCATED_PAYLOAD_BYTE: u8 = 0x80;

struct ScriptedInput {
    bytes: Cursor<Vec<u8>>,
    fail: bool,
}

impl ScriptedInput {
    fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Cursor::new(bytes),
            fail: false,
        }
    }

    fn failure() -> Self {
        Self {
            bytes: Cursor::new(Vec::new()),
            fail: true,
        }
    }
}

impl Read for ScriptedInput {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.fail {
            Err(std::io::ErrorKind::PermissionDenied.into())
        } else {
            self.bytes.read(output)
        }
    }
}

impl AsyncRead for ScriptedInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let destination = output.initialize_unfilled();
        match Read::read(&mut *self, destination) {
            Ok(length) => {
                output.advance(length);
                Poll::Ready(Ok(()))
            }
            Err(error) => Poll::Ready(Err(error)),
        }
    }
}

#[derive(Clone, Copy)]
struct ConditionalPayload {
    fail: bool,
}

impl Serialize for ConditionalPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.fail {
            Err(S::Error::custom(SERIALIZATION_FAILURE))
        } else {
            serializer.serialize_unit()
        }
    }
}

#[tokio::test]
async fn one_async_instantiation_covers_every_header_outcome() {
    let request = request();
    let mut valid = ScriptedInput::bytes(encode(&request).unwrap());
    assert_eq!(
        read_frame::<_, RequestEnvelope>(&mut valid).await.unwrap(),
        request
    );
    assert_end_of_stream(
        read_frame::<_, RequestEnvelope>(&mut ScriptedInput::bytes(Vec::new())).await,
    );
    assert_io(read_frame::<_, RequestEnvelope>(&mut ScriptedInput::failure()).await);
    assert_too_large(
        read_frame::<_, RequestEnvelope>(&mut ScriptedInput::bytes(oversized_header())).await,
    );
    assert_io(read_frame::<_, RequestEnvelope>(&mut ScriptedInput::bytes(truncated_frame())).await);
}

#[test]
fn one_blocking_instantiation_covers_every_header_outcome() {
    let request = request();
    let mut valid = ScriptedInput::bytes(encode(&request).unwrap());
    assert_eq!(
        read_frame_blocking::<_, RequestEnvelope>(&mut valid).unwrap(),
        request
    );
    assert_end_of_stream(read_frame_blocking::<_, RequestEnvelope>(
        &mut ScriptedInput::bytes(Vec::new()),
    ));
    assert_io(read_frame_blocking::<_, RequestEnvelope>(
        &mut ScriptedInput::failure(),
    ));
    assert_too_large(read_frame_blocking::<_, RequestEnvelope>(
        &mut ScriptedInput::bytes(oversized_header()),
    ));
    assert_io(read_frame_blocking::<_, RequestEnvelope>(
        &mut ScriptedInput::bytes(truncated_frame()),
    ));
}

#[tokio::test]
async fn async_writer_propagates_same_payload_serialization_failure() {
    let mut writer = tokio::io::sink();
    write_frame(&mut writer, &ConditionalPayload { fail: false })
        .await
        .unwrap();
    assert!(
        write_frame(&mut writer, &ConditionalPayload { fail: true })
            .await
            .is_err()
    );
}

#[test]
fn blocking_writer_propagates_same_payload_serialization_failure() {
    let mut writer = std::io::sink();
    write_frame_blocking(&mut writer, &ConditionalPayload { fail: false }).unwrap();
    assert!(write_frame_blocking(&mut writer, &ConditionalPayload { fail: true }).is_err());
}

fn request() -> RequestEnvelope {
    RequestEnvelope::new(REQUEST_ID, REQUEST_METHOD, Value::Null)
}

fn oversized_header() -> Vec<u8> {
    ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().to_vec()
}

fn truncated_frame() -> Vec<u8> {
    let mut frame = TRUNCATED_PAYLOAD_LENGTH.to_be_bytes().to_vec();
    frame.push(TRUNCATED_PAYLOAD_BYTE);
    frame
}

fn assert_end_of_stream<T>(result: Result<T, ProtocolError>) {
    assert!(matches!(result, Err(ProtocolError::EndOfStream)));
}

fn assert_io<T>(result: Result<T, ProtocolError>) {
    assert!(matches!(result, Err(ProtocolError::Io(_))));
}

fn assert_too_large<T>(result: Result<T, ProtocolError>) {
    assert!(matches!(result, Err(ProtocolError::FrameTooLarge { .. })));
}
