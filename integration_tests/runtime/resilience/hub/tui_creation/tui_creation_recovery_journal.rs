use super::constants::*;
use std::path::Path;
use yolo_hub_identity::HubId;
use yolo_tui_creation_journal::{
    CreationIntent, CreationJournal, JOURNAL_FILE, JOURNAL_SCHEMA_VERSION,
};
use yolo_tui_tab::{TabDocument, TabStore};

/// The TUI holds the journal lock while live. Inspect its bounded durable JSON
/// without taking a competing journal owner or creating any missing document.
pub fn before_create(directory: &Path, intent: &CreationIntent) -> bool {
    let Ok(document) =
        yolo_runtime::paths::read_json::<serde_json::Value>(&directory.join(JOURNAL_FILE))
    else {
        return false;
    };
    if document
        .get(SCHEMA_FIELD)
        .and_then(serde_json::Value::as_u64)
        != Some(u64::from(JOURNAL_SCHEMA_VERSION))
    {
        return false;
    }
    let entry = document
        .get(ENTRY_FIELD)
        .cloned()
        .and_then(|value| serde_json::from_value::<HubId>(value).ok());
    let intents = document
        .get(INTENTS_FIELD)
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<CreationIntent>>(value).ok());
    entry == Some(intent.origin)
        && intents.as_deref() == Some(std::slice::from_ref(intent))
        && document
            .get(PENDING_FIELD)
            .and_then(serde_json::Value::as_array)
            .is_some_and(Vec::is_empty)
}

pub fn cleared(directory: &Path) -> bool {
    let Ok(document) =
        yolo_runtime::paths::read_json::<serde_json::Value>(&directory.join(JOURNAL_FILE))
    else {
        return false;
    };
    document
        .get(SCHEMA_FIELD)
        .and_then(serde_json::Value::as_u64)
        == Some(u64::from(JOURNAL_SCHEMA_VERSION))
        && [INTENTS_FIELD, PENDING_FIELD].into_iter().all(|field| {
            document
                .get(field)
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
        })
}

pub fn retained(directory: &Path, entry: HubId, expected: &CreationIntent) -> TabDocument {
    let store = TabStore::open(directory, entry).unwrap();
    let journal = CreationJournal::open(store.directory().unwrap(), entry).unwrap();
    assert_eq!(journal.intents().unwrap(), vec![expected.clone()]);
    assert!(journal.pending().unwrap().is_empty());
    let document = store.document().clone();
    unbound(&document);
    document
}

pub fn settled(directory: &Path, entry: HubId) {
    let store = TabStore::open(directory, entry).unwrap();
    let journal = CreationJournal::open(store.directory().unwrap(), entry).unwrap();
    assert!(journal.intents().unwrap().is_empty());
    assert!(journal.pending().unwrap().is_empty());
    unbound(store.document());
}

pub fn unbound(document: &TabDocument) {
    assert!(document.tabs.entries().iter().all(|workspace| {
        workspace
            .view            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    }));
}
