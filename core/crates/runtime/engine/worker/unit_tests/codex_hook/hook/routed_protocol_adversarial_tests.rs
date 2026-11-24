use super::super::constants::{
    CODEX_FRAME_HEADER_BYTES, CODEX_MIN_FRAME_BYTES, CODEX_REQUEST_TIMEOUT,
};
use super::super::protocol::{CodexIngressRequest, read_request};
use super::ingress_test_assets::IngressTestAssets;
use super::support::{FIRST_EVENT_SEQUENCE, Fixture};
use super::transport_support::write_bytes_frame;
use crate::codex_hooks::MAX_INPUT_BYTES;
use crate::local_unix::LocalUnixStream;
use tokio::io::AsyncWriteExt;

const TEST_ROOT_DIRECTORY: &str = "routed-protocol-assets";
const EVENT_PADDING: &str = "p";
const TRUNCATED_PAYLOAD: &[u8] = b"{";
const TRUNCATED_REMAINDER_BYTES: usize = 1;

#[tokio::test]
async fn native_and_event_frames_have_exact_distinct_shapes() {
    let fixture = Fixture::new();
    let directory = tempfile::tempdir().unwrap();
    let assets = test_assets(directory.path());
    let begin = native_request(&assets, &fixture);
    let event = fixture.start(FIRST_EVENT_SEQUENCE);

    assert!(matches!(
        decode(&yolo_capnp_codec::encode(&begin).unwrap()).await.unwrap(),
        CodexIngressRequest::Native(decoded) if decoded == begin
    ));
    assert!(matches!(
        decode(&yolo_capnp_codec::encode(&event).unwrap()).await.unwrap(),
        CodexIngressRequest::Event(decoded) if decoded == event
    ));
}

#[tokio::test]
async fn provider_input_limit_does_not_become_the_event_frame_limit() {
    let fixture = Fixture::new();
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.provider_session_id = EVENT_PADDING.repeat(MAX_INPUT_BYTES);
    let payload = yolo_capnp_codec::encode(&event).unwrap();
    assert!(payload.len() > MAX_INPUT_BYTES);

    assert!(matches!(
        decode(&payload).await.unwrap(),
        CodexIngressRequest::Event(decoded) if decoded == event
    ));
}

#[tokio::test]
async fn oversized_native_input_is_rejected_after_unique_shape_detection() {
    let fixture = Fixture::new();
    let directory = tempfile::tempdir().unwrap();
    let assets = test_assets(directory.path());
    let mut begin = native_request(&assets, &fixture);
    if let crate::codex_hooks::CodexHookInput::SessionStart { model, .. } = &mut begin.input {
        *model = EVENT_PADDING.repeat(MAX_INPUT_BYTES);
    }
    let payload = yolo_capnp_codec::encode(&begin).unwrap();
    assert!(payload.len() > MAX_INPUT_BYTES);

    assert!(decode(&payload).await.is_err());
}

#[tokio::test]
async fn mixed_native_and_event_fields_are_not_guessed_as_either_request() {
    let fixture = Fixture::new();
    let directory = tempfile::tempdir().unwrap();
    let assets = test_assets(directory.path());
    let begin = native_request(&assets, &fixture);
    let mut mixed = serde_json::to_value(begin).unwrap();
    let event = serde_json::to_value(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();
    mixed
        .as_object_mut()
        .unwrap()
        .extend(event.as_object().unwrap().clone());

    assert!(decode(&yolo_capnp_codec::encode(&mixed).unwrap()).await.is_err());
}

#[tokio::test]
async fn zero_length_and_truncated_frames_fail_closed() {
    let zero_length = CODEX_MIN_FRAME_BYTES.saturating_sub(1);
    assert_eq!(zero_length, usize::default());
    assert!(decode_declared(&[], zero_length).await.is_err());
    assert!(
        decode_declared(
            TRUNCATED_PAYLOAD,
            TRUNCATED_PAYLOAD.len() + TRUNCATED_REMAINDER_BYTES,
        )
        .await
        .is_err()
    );
}

async fn decode(payload: &[u8]) -> anyhow::Result<CodexIngressRequest> {
    let (mut server, mut client) = LocalUnixStream::pair()?;
    tokio::time::timeout(CODEX_REQUEST_TIMEOUT, async {
        let writer = async {
            write_bytes_frame(&mut client, payload).await;
            client.shutdown().await.unwrap();
        };
        let (_, result) = tokio::join!(writer, read_request(&mut server));
        result
    })
    .await?
}

async fn decode_declared(
    payload: &[u8],
    declared_length: usize,
) -> anyhow::Result<CodexIngressRequest> {
    let (mut server, mut client) = LocalUnixStream::pair()?;
    let length = u32::try_from(declared_length)?;
    let header = length.to_be_bytes();
    assert_eq!(header.len(), CODEX_FRAME_HEADER_BYTES);
    client.write_all(&header).await?;
    client.write_all(payload).await?;
    client.shutdown().await?;
    read_request(&mut server).await
}

fn test_assets(root: &std::path::Path) -> IngressTestAssets {
    let root = root.join(TEST_ROOT_DIRECTORY);
    std::fs::create_dir(&root).unwrap();
    IngressTestAssets::new(&root)
}

#[path = "../../../test_support/unit/codex_hook/native/native_protocol.rs"]
mod native_support;
use native_support::native_request;
