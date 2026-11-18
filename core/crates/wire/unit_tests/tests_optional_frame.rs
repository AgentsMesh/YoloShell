use super::*;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, ReadBuf};
use yolo_capnp_value::Value;

const REQUEST_ID: &str = "optional-frame-request";
const REQUEST_METHOD: &str = "optional-frame-method";
const OPTIONAL_FRAME_TEST_TIMEOUT: Duration = Duration::from_secs(1);
const OPTIONAL_FRAME_TIMEOUT_FAILURE: &str = "optional frame read exceeded its test timeout";
const INVALID_MESSAGEPACK_BYTE: u8 = 0xc1;
const SINGLE_BYTE_COUNT: usize = std::mem::size_of::<u8>();

struct OneByteInput(Cursor<Vec<u8>>);
struct FailingInput;

impl AsyncRead for OneByteInput {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let source = self.0.get_ref();
        let position = self.0.position() as usize;
        if let Some(byte) = source.get(position).copied() {
            output.put_slice(&[byte]);
            self.0.set_position((position + SINGLE_BYTE_COUNT) as u64);
        }
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for FailingInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(std::io::ErrorKind::PermissionDenied.into()))
    }
}

#[tokio::test]
async fn optional_reader_returns_typed_clean_eof() {
    let mut input = OneByteInput(Cursor::new(Vec::new()));
    let result = bounded_read(&mut input).await.unwrap();
    assert_eq!(result, OptionalFrameRead::CleanEof);
}

#[tokio::test]
async fn optional_reader_accepts_fragmented_valid_frame() {
    let request = request();
    let mut input = OneByteInput(Cursor::new(encode(&request).unwrap()));
    let result = bounded_read(&mut input).await.unwrap();
    assert_eq!(result, OptionalFrameRead::Frame(request));
}

#[tokio::test]
async fn optional_reader_rejects_every_partial_header_length() {
    for received in SINGLE_BYTE_COUNT..FRAME_LENGTH_BYTES {
        let header = (MAX_FRAME_BYTES as u32).to_be_bytes();
        let mut input = OneByteInput(Cursor::new(header[..received].to_vec()));
        assert!(matches!(
            bounded_read(&mut input).await,
            Err(ProtocolError::TruncatedHeader { received: actual }) if actual == received
        ));
    }
}

#[tokio::test]
async fn optional_reader_keeps_non_header_failures_distinct() {
    assert!(matches!(
        bounded_read(&mut FailingInput).await,
        Err(ProtocolError::Io(error))
            if error.kind() == std::io::ErrorKind::PermissionDenied
    ));

    let mut oversized = OneByteInput(Cursor::new(
        ((MAX_FRAME_BYTES + SINGLE_BYTE_COUNT) as u32)
            .to_be_bytes()
            .to_vec(),
    ));
    assert!(matches!(
        bounded_read(&mut oversized).await,
        Err(ProtocolError::FrameTooLarge { .. })
    ));

    let payload_length = u32::try_from(FRAME_LENGTH_BYTES).unwrap();
    let mut truncated = OneByteInput(Cursor::new(payload_length.to_be_bytes().to_vec()));
    assert!(matches!(
        bounded_read(&mut truncated).await,
        Err(ProtocolError::Io(error))
            if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));

    let mut invalid = OneByteInput(Cursor::new(frame(&[INVALID_MESSAGEPACK_BYTE])));
    assert!(matches!(
        bounded_read(&mut invalid).await,
        Err(ProtocolError::Decode(_))
    ));
}

async fn bounded_read<R>(
    reader: &mut R,
) -> Result<OptionalFrameRead<RequestEnvelope>, ProtocolError>
where
    R: AsyncRead + Unpin,
{
    tokio::time::timeout(OPTIONAL_FRAME_TEST_TIMEOUT, read_optional_frame(reader))
        .await
        .expect(OPTIONAL_FRAME_TIMEOUT_FAILURE)
}

fn request() -> RequestEnvelope {
    RequestEnvelope::new(REQUEST_ID, REQUEST_METHOD, Value::Null)
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let length = u32::try_from(payload.len()).unwrap();
    let mut frame = length.to_be_bytes().to_vec();
    frame.extend_from_slice(payload);
    frame
}
