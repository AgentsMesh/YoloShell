use crate::{
    FIRST_SSH_PROFILE_REVISION, SshProfileConfiguration, SshProfileState, SshProfileStore,
    constants::*, record::Record,
};
use anyhow::{Context, Result, ensure};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

impl SshProfileStore {
    pub fn open(
        directory: PrivateDirectory,
        hub: PeerIdentity,
        seed: Option<SshProfileConfiguration>,
    ) -> Result<Self> {
        if let Some(seed) = &seed {
            seed.validate()?;
        }
        let object = directory.durable_identity()?;
        let path = directory.path().join(PROFILE_FILE);
        let lock_path = directory.path().join(LOCK_FILE);
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        namespace.verify()?;
        let lock = CoordinationLock::try_acquire(&lock_path, LOCK_BYTES)?.context(BUSY)?;
        namespace.verify()?;
        let (record, file) = if namespace.file_is_missing(&path)? {
            let state = SshProfileState {
                hub,
                revision: FIRST_SSH_PROFILE_REVISION,
                last_change: Uuid::new_v4(),
                configuration: seed.clone().unwrap_or_default(),
            };
            let record = Record {
                schema_version: RECORD_SCHEMA,
                directory: object,
                state,
            };
            record.validate(object, hub)?;
            let file =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    namespace.verify()?;
                    lock.verify()
                })?;
            (record, file)
        } else {
            namespace.read_json_with_identity::<Record>(&path)?
        };
        record.validate(object, hub)?;
        ensure!(
            seed.is_none_or(|seed| seed == record.state.configuration),
            SEED_CONFLICT
        );
        let store = Self {
            record,
            path,
            namespace,
            lock,
            file,
            lost: AtomicBool::new(false),
        };
        store.verify()?;
        Ok(store)
    }
}
