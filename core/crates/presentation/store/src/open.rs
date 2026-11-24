use crate::{LayoutStore, StoreError, constants::*, record::Record};
use anyhow::Context;
use std::sync::atomic::AtomicBool;
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

impl LayoutStore {
    pub fn open(directory: PrivateDirectory, hub_id: HubId) -> Result<Self, StoreError> {
        directory.verify_path()?;
        let path = directory.path().join(LAYOUT_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        directory.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (record, identity) = if namespace.file_is_missing(&path)? {
            let record = Record::empty(hub_id);
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    lock.verify()
                })?;
            (record, identity)
        } else {
            namespace.read_json_with_identity::<Record>(&path)?
        };
        record.validate(hub_id)?;
        let store = Self {
            namespace,
            lock,
            path,
            identity,
            record,
            lost: AtomicBool::new(false),
        };
        store.check()?;
        Ok(store)
    }
}
