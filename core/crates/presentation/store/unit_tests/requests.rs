use super::{fixture::*, *};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_presentation_protocol::ProtocolError;

#[test]
fn malformed_or_foreign_requests_do_not_poison_valid_service() {
    let (_root, _path, hub, mut store) = fixture();
    assert!(store.list().unwrap().is_empty());
    let valid = update(hub);
    let mut foreign = valid.clone();
    foreign.address.hub_id = HubId::new();
    assert!(matches!(
        store.get(foreign.address),
        Err(StoreError::Protocol(ProtocolError::AddressMismatch))
    ));
    assert!(matches!(
        store.update(foreign),
        Err(StoreError::Protocol(ProtocolError::AddressMismatch))
    ));
    let mut invalid = valid.clone();
    invalid.address.layout_id = Uuid::nil();
    assert!(matches!(
        store.get(invalid.address),
        Err(StoreError::Protocol(ProtocolError::InvalidId))
    ));
    invalid = valid.clone();
    invalid.request_id = Uuid::nil();
    assert!(matches!(
        store.update(invalid),
        Err(StoreError::Protocol(ProtocolError::InvalidId))
    ));
    assert!(store.get(valid.address).unwrap().is_none());
    let first = store.update(valid.clone()).unwrap();
    assert_eq!(store.list().unwrap(), vec![first.clone()]);
    assert!(matches!(
        store.update(valid.clone()),
        Err(StoreError::Protocol(ProtocolError::RevisionConflict))
    ));
    assert_eq!(store.get(valid.address).unwrap(), Some(first));
}

#[test]
fn document_capacity_is_bounded_and_existing_documents_remain_editable() {
    let (_root, _path, hub, mut store) = fixture();
    let first = store.update(update(hub)).unwrap();
    for _ in 1..MAX_DOCUMENTS {
        store.update(update(hub)).unwrap();
    }
    assert!(matches!(
        store.update(update(hub)),
        Err(StoreError::Protocol(ProtocolError::Limit))
    ));
    let mut edit = update(hub);
    edit.address = first.address;
    edit.expected_revision = first.revision;
    edit.tabs.clear();
    let closed = store.update(edit).unwrap();
    assert!(closed.tabs.is_empty());
}
