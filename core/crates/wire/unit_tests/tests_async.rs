use super::*;
use yolo_capnp_value::Value;

#[tokio::test]
async fn async_round_trip() {
    let (mut left, mut right) = tokio::io::duplex(1024);
    let request = RequestEnvelope::new("r", method::LIST_SESSIONS, Value::Null);
    let expected = request.clone();
    let writer = tokio::spawn(async move { write_frame(&mut left, &request).await.unwrap() });
    let decoded: RequestEnvelope = read_frame(&mut right).await.unwrap();
    writer.await.unwrap();
    assert_eq!(decoded, expected);
}

#[tokio::test]
async fn reader_reports_eof_oversized_and_truncated_frames() {
    let mut empty = tokio::io::empty();
    assert!(matches!(
        read_frame::<_, RequestEnvelope>(&mut empty).await,
        Err(ProtocolError::EndOfStream)
    ));
    let mut oversized = std::io::Cursor::new(((MAX_FRAME_BYTES + 1) as u32).to_be_bytes());
    assert!(matches!(
        read_frame::<_, RequestEnvelope>(&mut oversized).await,
        Err(ProtocolError::FrameTooLarge { .. })
    ));
    let mut truncated = std::io::Cursor::new([0, 0, 0, 3, 0x80]);
    assert!(matches!(
        read_frame::<_, RequestEnvelope>(&mut truncated).await,
        Err(ProtocolError::Io(_))
    ));
}

#[tokio::test]
async fn closed_peer_and_non_eof_header_failures_are_io_errors() {
    let (mut writer, reader) = tokio::io::duplex(8);
    drop(reader);
    let request = RequestEnvelope::new("id", method::HELLO, Value::Null);
    assert!(matches!(
        write_frame(&mut writer, &request).await,
        Err(ProtocolError::Io(_))
    ));

    use std::pin::Pin;
    use std::task::{Context, Poll};
    struct FailingReader;
    impl tokio::io::AsyncRead for FailingReader {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Poll::Ready(Err(std::io::ErrorKind::PermissionDenied.into()))
        }
    }
    assert!(matches!(
        read_frame::<_, RequestEnvelope>(&mut FailingReader).await,
        Err(ProtocolError::Io(_))
    ));
}
