use crate::{ClientAccessStore, constants::*, record::Record};
use anyhow::{Context, Result, ensure};
use std::sync::atomic::AtomicBool;
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, FIRST_ACCESS_REVISION,
};
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

impl ClientAccessStore {
    /// `directory` is the prepared private client-access directory, not its parent.
    /// Existing authorization is never silently replaced by a supplied seed.
    pub fn open(
        directory: PrivateDirectory,
        entry_hub: HubId,
        seed: Option<AccessConfiguration>,
    ) -> Result<Self> {
        if let Some(seed) = &seed {
            seed.validate()?;
            ensure!(seed.entry_hub == entry_hub, WRONG_IDENTITY);
        }
        let path = directory.path().join(RECORD_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        let object = directory.durable_identity()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (record, file_identity) = if namespace.file_is_missing(&path)? {
            let record = Record {
                schema_version: PRIVATE_RECORD_SCHEMA,
                directory: object,
                configuration: seed.clone().unwrap_or(AccessConfiguration {
                    schema_version: ACCESS_SCHEMA_VERSION,
                    entry_hub,
                    revision: FIRST_ACCESS_REVISION,
                    clients: Vec::new(),
                }),
            };
            record.validate(object, entry_hub)?;
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    lock.verify()?;
                    namespace.verify()
                })?;
            (record, identity)
        } else {
            namespace.read_json_with_identity(&path)?
        };
        record.validate(object, entry_hub)?;
        ensure!(
            seed.as_ref()
                .is_none_or(|seed| *seed == record.configuration),
            SEED_CONFLICT
        );
        let store = Self {
            entry_hub,
            directory: object,
            path,
            namespace,
            lock,
            file_identity,
            configuration: record.configuration,
            lost: AtomicBool::new(false),
        };
        store.current()?;
        Ok(store)
    }
}
