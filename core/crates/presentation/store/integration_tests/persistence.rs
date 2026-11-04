use super::{fixture::*, *};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;
use yolo_presentation_protocol::ProtocolError;

#[test]
fn listing_is_stable_by_address_independently_of_creation_order() {
    let (_root, path, hub, mut store) = fixture();
    let mut requests = [update(hub), update(hub)];
    requests.sort_by_key(|request| std::cmp::Reverse(request.address));
    let mut expected = requests
        .map(|request| store.update(request).unwrap())
        .to_vec();
    expected.sort_by_key(|document| document.address);
    assert_eq!(store.list().unwrap(), expected);
    drop(store);
    let reopened = LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).unwrap();
    assert_eq!(reopened.list().unwrap(), expected);
}

#[test]
fn restart_preserves_bindings_and_resolves_lost_update_reply_by_request_id() {
    let (_root, path, hub, mut store) = fixture();
    let request = update(hub);
    let first = store.update(request.clone()).unwrap();
    drop(store);
    let mut reopened = LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).unwrap();
    assert_eq!(reopened.get(first.address).unwrap(), Some(first.clone()));
    assert_eq!(first.last_request_id, request.request_id);
    assert!(matches!(
        reopened.update(request.clone()),
        Err(StoreError::Protocol(ProtocolError::RevisionConflict))
    ));
    let mut close = request;
    close.request_id = Uuid::new_v4();
    close.expected_revision = first.revision;
    close.tabs.clear();
    let closed = reopened.update(close).unwrap();
    assert!(closed.tabs.is_empty());
    drop(reopened);
    let restored = LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).unwrap();
    assert_eq!(restored.get(first.address).unwrap(), Some(closed));
}

#[test]
fn concurrent_clients_cannot_overwrite_the_winner_with_a_stale_revision() {
    let (_root, _path, hub, mut store) = fixture();
    let first = store.update(update(hub)).unwrap();
    let store = Arc::new(Mutex::new(store));
    let handles: Vec<_> = [TITLE, NEXT_TITLE]
        .into_iter()
        .map(|title| {
            let mut edit = update(hub);
            edit.address = first.address;
            edit.expected_revision = first.revision;
            edit.tabs[0].title = title.into();
            let store = Arc::clone(&store);
            std::thread::spawn(move || store.lock().unwrap().update(edit))
        })
        .collect();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| matches!(
                value,
                Err(StoreError::Protocol(ProtocolError::RevisionConflict))
            ))
            .count(),
        1
    );
}

#[test]
fn another_writer_or_hub_cannot_adopt_the_store() {
    let (_root, path, _hub, store) = fixture();
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), HubId::new()).is_err());
    drop(store);
    assert!(LayoutStore::open(PrivateDirectory::open(&path).unwrap(), HubId::new()).is_err());
}
