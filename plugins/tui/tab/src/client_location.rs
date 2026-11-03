use crate::{
    TabStore, client_registry, constants::*, default_directory,
    store::is_document_decode_error,
};
use anyhow::{Result, bail};
use std::path::Path;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};
use yolo_tui_creation_journal::{CreationJournal, JOURNAL_FILE};

pub fn open_client(hub: HubId) -> Result<TabStore> {
    open_client_in(&default_directory(hub)?, hub)
}

/// Embedding/test hook; the public CLI chooses its state location internally.
#[doc(hidden)]
pub fn open_client_in(base: &Path, hub: HubId) -> Result<TabStore> {
    let base = yolo_paths::platform::platform_path(base);
    let parent = PrivateDirectory::ensure(&base)?;
    let candidates = client_registry::children(&parent, hub, true)?;
    let mut fallback = match TabStore::try_open_retained(parent.try_clone()?, hub) {
        Ok(store) => store,
        Err(error) if is_document_decode_error(&error) => None,
        Err(error) => return Err(error),
    };
    if fallback
        .as_ref()
        .map(|store| pending(store, hub))
        .transpose()?
        .unwrap_or(false)
    {
        return Ok(fallback.take().expect(ALLOCATION_FAILED));
    }
    for candidate in candidates {
        let directory = parent.open_descendant(&candidate.path)?;
        let store = match TabStore::try_open_retained(directory, hub) {
            Ok(store) => store,
            Err(error) if is_document_decode_error(&error) => continue,
            Err(error) => return Err(error),
        };
        let Some(store) = store else {
            continue;
        };
        if pending(&store, hub)? {
            return Ok(store);
        }
        if fallback.is_none() {
            fallback = Some(store);
        }
    }
    if let Some(store) = fallback {
        return Ok(store);
    }
    let clients = parent.ensure_descendant(&parent.path().join(CLIENTS_DIRECTORY))?;
    for _ in 0..ALLOCATION_ATTEMPTS {
        let id = Uuid::new_v4();
        if id == hub.as_uuid() {
            continue;
        }
        let path = clients.path().join(id.to_string());
        clients.verify_path()?;
        let directory = clients.ensure_descendant(&path)?;
        match TabStore::try_open_retained(directory, hub) {
            Ok(Some(store)) => {
                parent.verify_path()?;
                clients.verify_path()?;
                return Ok(store);
            }
            Ok(None) => {}
            Err(error) if is_document_decode_error(&error) => {}
            Err(error) => return Err(error),
        }
    }
    bail!(ALLOCATION_FAILED)
}

fn pending(store: &TabStore, hub: HubId) -> Result<bool> {
    let directory = store.directory()?;
    let path = directory.path().join(JOURNAL_FILE);
    let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory.try_clone()?)])?;
    if namespace.file_is_missing(&path)? {
        return Ok(false);
    }
    let journal = CreationJournal::open(directory, hub)?;
    Ok(!journal.intents()?.is_empty() || !journal.pending()?.is_empty())
}
