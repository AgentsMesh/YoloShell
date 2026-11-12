use crate::{CreationJournal, constants::*, record::Document};
use anyhow::{Context, Result};
use std::sync::atomic::AtomicBool;
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

impl CreationJournal {
    pub fn open(parent: PrivateDirectory, entry: HubId) -> Result<Self> {
        parent.verify_path()?;
        let directory = parent.durable_identity()?;
        let path = parent.path().join(JOURNAL_FILE);
        let lock = CoordinationLock::try_acquire(&parent.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        parent.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, parent)])?;
        let (document, identity) = if namespace.file_is_missing(&path)? {
            let document = Document {
                schema_version: JOURNAL_SCHEMA_VERSION,
                entry_hub: entry,
                directory,
                revision: FIRST_REVISION,
                pending: Vec::new(),
                intents: Vec::new(),
            };
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &document, || {
                    lock.verify()
                })?;
            (document, identity)
        } else {
            namespace.read_json_with_identity::<Document>(&path)?
        };
        document.validate(entry, directory)?;
        let journal = Self {
            namespace,
            lock,
            path,
            identity,
            document,
            lost: AtomicBool::new(false),
            recovery: None,
        };
        journal.verify()?;
        Ok(journal)
    }
}
