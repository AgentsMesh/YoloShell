use super::*;
use crate::protocol::event;

const SUCCESS_NEXT_SEQUENCE: u64 = 1;

#[tokio::test]
async fn valid_event_restores_the_attachment_stream_for_the_next_read() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let (stream, peer) = crate::attachment_test_support::connected().await;
    let event = EventEnvelope::new(
        event::SESSION_UPDATED,
        Some(terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &(),
    )
    .unwrap();
    peer.event(&event).await.unwrap();
    let mut attachment = Attachment {
        stream: Some(stream),
        terminal_id,
        runtime_epoch,
        initial: AttachResult {
            runtime_epoch,
            snapshot: None,
            chunks: Vec::new(),
            next_sequence: SUCCESS_NEXT_SEQUENCE,
            resynchronized: false,
        },
    };

    let received = attachment.next_event().await.unwrap();

    assert_eq!(received.event, event::SESSION_UPDATED);
    assert!(attachment.stream.is_some());
}
