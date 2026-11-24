use super::*;
use crate::protocol::ProtocolError;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const DUPLEX_CAPACITY: usize = 4 * 1_024;
const RESERVED_MESSAGEPACK_BYTE: u8 = 0xc1;

#[tokio::test]
async fn dedicated_frame_round_trips_a_begin_request() {
    let request = request();
    let (mut writer, mut reader) = tokio::io::duplex(DUPLEX_CAPACITY);
    let expected = request.clone();
    let task = tokio::spawn(async move { write_frame(&mut writer, &request).await.unwrap() });
    let decoded: ClaudeHookRequest = read_frame(&mut reader).await.unwrap();
    task.await.unwrap();
    assert_eq!(decoded, expected);
}

#[tokio::test]
async fn oversized_frame_is_rejected_before_payload_allocation() {
    let length = MAX_CLAUDE_HOOK_FRAME_BYTES + 1;
    let (mut writer, mut reader) = tokio::io::duplex(DUPLEX_CAPACITY);
    writer
        .write_all(&u32::try_from(length).unwrap().to_be_bytes())
        .await
        .unwrap();
    let result = read_frame::<_, ClaudeHookRequest>(&mut reader).await;
    assert!(
        matches!(result, Err(ProtocolError::FrameTooLarge { length: value }) if value == length)
    );
}

#[tokio::test]
async fn malformed_and_empty_frames_fail_closed() {
    let (mut writer, mut reader) = tokio::io::duplex(DUPLEX_CAPACITY);
    writer.write_all(&1_u32.to_be_bytes()).await.unwrap();
    writer
        .write_all(&[RESERVED_MESSAGEPACK_BYTE])
        .await
        .unwrap();
    assert!(matches!(
        read_frame::<_, ClaudeHookRequest>(&mut reader).await,
        Err(ProtocolError::Decode(_))
    ));

    let (writer, mut reader) = tokio::io::duplex(DUPLEX_CAPACITY);
    drop(writer);
    assert!(matches!(
        read_frame::<_, ClaudeHookRequest>(&mut reader).await,
        Err(ProtocolError::EndOfStream)
    ));
}

fn request() -> ClaudeHookRequest {
    ClaudeHookRequest {
        schema_version: CLAUDE_HOOK_SCHEMA_VERSION,
        credentials: ClaudeHookCredentials {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            invocation_id: Uuid::new_v4(),
            hook_token: Uuid::new_v4(),
            helper_capability: Uuid::new_v4(),
        },
        operation: ClaudeHookOperation::Begin {
            provider_session_id: Uuid::new_v4().to_string(),
            discover_session: false,
            mode: ClaudeInvocationMode::Fresh,
            cwd: PathBuf::from("/tmp/project"),
            data_home: PathBuf::from("/tmp/claude"),
        },
    }
}
