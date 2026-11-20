use super::test_support::{Fixture, request};
use crate::codex_hooks::{
    CODEX_NATIVE_MAX_REQUEST_BYTES, read_native_request, write_native_request,
};
use tokio::io::AsyncWriteExt;

const PIPE_CAPACITY: usize = 8 * 1024;
const OVERSIZED_OFFSET: usize = 1;
const INVALID_JSON: &[u8] = b"{}";

#[tokio::test]
async fn native_callback_frames_preserve_the_event_occurrence_exactly() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let (mut writer, mut reader) = tokio::io::duplex(PIPE_CAPACITY);
    write_native_request(&mut writer, &callback).await.unwrap();
    assert_eq!(read_native_request(&mut reader).await.unwrap(), callback);
}

#[tokio::test]
async fn native_callback_rejects_oversized_headers_before_reading_payload() {
    let (mut writer, mut reader) = tokio::io::duplex(PIPE_CAPACITY);
    let length = u32::try_from(CODEX_NATIVE_MAX_REQUEST_BYTES + OVERSIZED_OFFSET).unwrap();
    writer.write_all(&length.to_be_bytes()).await.unwrap();
    assert!(read_native_request(&mut reader).await.is_err());
}

#[tokio::test]
async fn native_callback_rejects_invalid_payload_and_truncated_streams() {
    let (mut writer, mut reader) = tokio::io::duplex(PIPE_CAPACITY);
    writer
        .write_all(&u32::try_from(INVALID_JSON.len()).unwrap().to_be_bytes())
        .await
        .unwrap();
    writer.write_all(INVALID_JSON).await.unwrap();
    assert!(read_native_request(&mut reader).await.is_err());
    drop(writer);
    assert!(read_native_request(&mut reader).await.is_err());
}

#[tokio::test]
async fn native_callback_cannot_encode_an_unscoped_event() {
    let fixture = Fixture::new();
    let mut callback = request(&fixture);
    callback.event_id = uuid::Uuid::nil();
    let (mut writer, _reader) = tokio::io::duplex(PIPE_CAPACITY);
    assert!(write_native_request(&mut writer, &callback).await.is_err());
}
