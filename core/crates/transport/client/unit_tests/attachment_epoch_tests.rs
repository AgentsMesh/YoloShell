use super::*;
use crate::model::OutputChunk;
use crate::protocol::{ProtocolError, ResponseEnvelope, event};
use std::time::Duration;
use tokio::net::UnixListener;

const ATTACH_AFTER_SEQUENCE: u64 = 0;
const OUTPUT_SEQUENCE: u64 = 1;
const ATTACHMENT_SOCKET_NAME: &str = "attachment-epoch.sock";
const EVENT_READ_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
enum EpochCorruption {
    BufferedOldRuntime,
    ChunkRuntime,
}

#[tokio::test]
async fn attachment_epoch_local_rejects_old_or_mismatched_output_and_poisons() {
    for corruption in [
        EpochCorruption::BufferedOldRuntime,
        EpochCorruption::ChunkRuntime,
    ] {
        assert_local_epoch_rejection(corruption).await;
    }
}

async fn assert_local_epoch_rejection(corruption: EpochCorruption) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join(ATTACHMENT_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let terminal_id = Uuid::new_v4();
    let expected_epoch = Uuid::new_v4();
    let old_epoch = Uuid::new_v4();
    let event_epoch = match corruption {
        EpochCorruption::BufferedOldRuntime => old_epoch,
        EpochCorruption::ChunkRuntime => expected_epoch,
    };
    let chunk_epoch = old_epoch;
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = crate::rpc_test_support::Peer::new(stream);
        let request = stream.request().await.unwrap();
        let initial = AttachResult {
            runtime_epoch: expected_epoch,
            snapshot: None,
            chunks: Vec::new(),
            next_sequence: OUTPUT_SEQUENCE,
            resynchronized: false,
        };
        let response = ResponseEnvelope::success(&request, &initial).unwrap();
        stream.respond(&response).await.unwrap();
        let chunk = OutputChunk {
            runtime_epoch: chunk_epoch,
            sequence: OUTPUT_SEQUENCE,
            data: Vec::new(),
        };
        let event = EventEnvelope::new(
            event::TERMINAL_OUTPUT,
            Some(terminal_id.to_string()),
            Some(event_epoch.to_string()),
            &chunk,
        )
        .unwrap();
        stream.event(&event).await.unwrap();
        stream.request().await
    });
    let client = Client::new(socket);
    let mut attachment = client
        .attach(terminal_id, ATTACH_AFTER_SEQUENCE)
        .await
        .unwrap();
    assert_eq!(attachment.runtime_epoch(), expected_epoch);
    assert!(matches!(
        attachment.next_event().await,
        Err(ClientError::Protocol(ProtocolError::EventMismatch))
    ));
    assert!(matches!(
        attachment.next_event().await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

#[tokio::test]
async fn cancelled_pending_event_read_closes_and_poisons_attachment() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let (stream, mut peer) = crate::attachment_test_support::connected().await;
    let mut attachment = Attachment {
        stream: Some(stream),
        terminal_id,
        runtime_epoch,
        initial: AttachResult {
            runtime_epoch,
            snapshot: None,
            chunks: Vec::new(),
            next_sequence: OUTPUT_SEQUENCE,
            resynchronized: false,
        },
    };
    assert!(
        tokio::time::timeout(EVENT_READ_TIMEOUT, attachment.next_event())
            .await
            .is_err()
    );
    assert!(matches!(
        attachment.next_event().await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
    let closed = tokio::time::timeout(EVENT_READ_TIMEOUT, peer.request())
        .await
        .unwrap();
    assert!(matches!(closed, Err(ProtocolError::EndOfStream)));
}

#[tokio::test]
async fn detach_closes_only_the_subscription_and_poisoned_detach_fails_closed() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let (stream, mut peer) = crate::attachment_test_support::connected().await;
    let active_attachment = attachment_fixture(Some(stream), terminal_id, runtime_epoch);
    let closed = tokio::spawn(async move { peer.request().await });

    active_attachment.detach().await.unwrap();

    assert!(matches!(
        closed.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
    let poisoned = attachment_fixture(None, terminal_id, runtime_epoch);
    assert!(matches!(
        poisoned.detach().await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
}

fn attachment_fixture(
    stream: Option<yolo_runtime_rpc::RuntimeConnection>,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
) -> Attachment {
    Attachment {
        stream,
        terminal_id,
        runtime_epoch,
        initial: AttachResult {
            runtime_epoch,
            snapshot: None,
            chunks: Vec::new(),
            next_sequence: OUTPUT_SEQUENCE,
            resynchronized: false,
        },
    }
}
