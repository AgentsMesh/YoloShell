use super::*;
use crate::protocol::event;

const SUCCESS_REVISION: u64 = 7;

#[tokio::test]
async fn change_events_decode_with_their_revision() {
    let terminal_id = Uuid::new_v4();
    let (stream, peer) = crate::attachment_test_support::connected().await;
    let change = CatalogueChange {
        revision: SUCCESS_REVISION,
        terminal_id: Some(terminal_id),
    };
    let envelope = EventEnvelope::new(
        event::CATALOGUE_CHANGED,
        Some(terminal_id.to_string()),
        None,
        &change,
    )
    .unwrap();
    peer.event(&envelope).await.unwrap();
    let mut watch = CatalogueWatch {
        stream: Some(stream),
        start_revision: SUCCESS_REVISION - 1,
    };

    assert_eq!(watch.next_change().await.unwrap(), change);
    assert!(watch.stream.is_some());
}

#[tokio::test]
async fn unrelated_events_are_rejected() {
    let terminal_id = Uuid::new_v4();
    let (stream, peer) = crate::attachment_test_support::connected().await;
    let envelope = EventEnvelope::new(
        event::SESSION_UPDATED,
        Some(terminal_id.to_string()),
        None,
        &(),
    )
    .unwrap();
    peer.event(&envelope).await.unwrap();
    let mut watch = CatalogueWatch {
        stream: Some(stream),
        start_revision: SUCCESS_REVISION,
    };

    assert!(matches!(
        watch.next_change().await,
        Err(ClientError::Protocol(ProtocolError::EventMismatch))
    ));
}
