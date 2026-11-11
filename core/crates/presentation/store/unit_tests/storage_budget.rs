use super::{fixture::*, *};
use crate::{constants::TOO_LARGE, record::Record};
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_paths::directory::PrivateDirectory;
use yolo_presentation_layout::{Axis, Layout, Tree};
use yolo_presentation_protocol::{LayoutUpdate, MAX_DOCUMENT_PANES, MAX_TITLE_BYTES, SharedTab};

const CHILDREN: usize = 2;

fn dense_tree(hub_id: HubId, panes: usize) -> Tree {
    if panes == 1 {
        return Layout::new(Some(TerminalAddress {
            hub_id,
            terminal_id: TerminalId::new(),
        }))
        .document()
        .root
        .clone();
    }
    Tree::Split {
        axis: Axis::Horizontal,
        ratio: Default::default(),
        first: Box::new(dense_tree(hub_id, panes / CHILDREN)),
        second: Box::new(dense_tree(hub_id, panes / CHILDREN)),
    }
}

fn dense_update(hub: HubId) -> LayoutUpdate {
    let mut request = update(hub);
    request.tabs = (0..MAX_DOCUMENT_PANES / CHILDREN / yolo_presentation_layout::MAX_PANES)
        .map(|_| {
            SharedTab::new(
                Uuid::new_v4(),
                "x".repeat(MAX_TITLE_BYTES),
                dense_tree(hub, yolo_presentation_layout::MAX_PANES),
            )
            .unwrap()
        })
        .collect();
    request.validate().unwrap();
    request
}

#[test]
fn aggregate_byte_budget_rejects_valid_individual_documents_without_poisoning_existing_state() {
    let (_root, path, hub, store) = fixture();
    drop(store);
    let mut record = Record::empty(hub);
    while serde_json::to_vec_pretty(&record).unwrap().len() as u64
        <= yolo_paths::constants::MAX_DURABLE_JSON_BYTES
    {
        record
            .documents
            .push(dense_update(hub).apply(None).unwrap());
    }
    assert!(record.documents.len() < MAX_DOCUMENTS);
    assert!(
        serde_json::to_vec(&record).unwrap().len() as u64
            <= yolo_paths::constants::MAX_DURABLE_JSON_BYTES
    );
    assert_eq!(record.validate(hub).unwrap_err().to_string(), TOO_LARGE);
    let overflow = record.documents.pop().unwrap();
    record.validate(hub).unwrap();
    yolo_paths::atomic_write_json(&path.join(LAYOUT_FILE), &record).unwrap();
    let before = std::fs::read(path.join(LAYOUT_FILE)).unwrap();
    let mut store = LayoutStore::open(PrivateDirectory::open(&path).unwrap(), hub).unwrap();
    let rejected = LayoutUpdate {
        address: overflow.address,
        request_id: overflow.last_request_id,
        expected_revision: 0,
        tabs: overflow.tabs,
    };
    assert!(matches!(
        store.update(rejected),
        Err(StoreError::Storage(_))
    ));
    assert_eq!(std::fs::read(path.join(LAYOUT_FILE)).unwrap(), before);
    assert!(store.get(overflow.address).unwrap().is_none());
    assert_eq!(store.list().unwrap().len(), record.documents.len());
    let retained = &record.documents[0];
    let edited = store
        .update(LayoutUpdate {
            address: retained.address,
            request_id: Uuid::new_v4(),
            expected_revision: retained.revision,
            tabs: vec![],
        })
        .unwrap();
    assert!(edited.tabs.is_empty());
}
