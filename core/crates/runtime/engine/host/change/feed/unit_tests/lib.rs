use uuid::Uuid;
use yolo_capnp_rpc::Value;
use yolo_host_change_feed::*;
use yolo_runtime_protocol::CatalogueChange;
use yolo_wire::{EventEnvelope, event, method};

fn change(value: Value) -> CatalogueChange {
    let envelope: EventEnvelope = yolo_wire::from_value(value).unwrap();
    assert_eq!(envelope.event, event::CATALOGUE_CHANGED);
    yolo_wire::from_value(envelope.payload).unwrap()
}

#[test]
fn only_catalogue_changes_reach_the_feed() {
    assert!(changes_catalogue(method::CREATE_SESSION));
    assert!(changes_catalogue(method::ARCHIVE_SESSION));
    assert!(changes_catalogue(method::RECONCILE));
    assert!(!changes_catalogue(method::RENEW_WRITER));
    assert!(!changes_catalogue(method::ACQUIRE_WRITER));
    assert!(!changes_catalogue(method::TERMINAL_INPUT));
    assert!(!changes_catalogue(method::LIST_SESSIONS));
}

#[tokio::test]
async fn touch_streams_revision_stamped_changes() {
    let feed = ChangeFeed::new();
    let (start, mut source) = feed.subscribe();
    assert_eq!(start, 0);
    let terminal_id = Uuid::new_v4();
    feed.touch(Some(terminal_id));
    feed.touch(None);
    assert_eq!(feed.revision(), 2);
    let first = change(source.next().await.unwrap());
    assert_eq!(first.revision, 1);
    assert_eq!(first.terminal_id, Some(terminal_id));
    let second = change(source.next().await.unwrap());
    assert_eq!(second.revision, 2);
    assert_eq!(second.terminal_id, None);
}
