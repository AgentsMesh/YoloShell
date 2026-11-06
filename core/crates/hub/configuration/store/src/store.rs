use crate::{constants::*, record::Record};
use anyhow::{Context, Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_paths::{
    coordination::CoordinationLock,
    directory::PrivateDirectory,
    identity::{DurableObjectIdentity, FileIdentity},
    namespace_lease::NamespaceLease,
};

pub struct ConfigurationStore {
    pub(crate) identity: PeerIdentity,
    directory: DurableObjectIdentity,
    path: PathBuf,
    namespace: NamespaceLease,
    lock: CoordinationLock,
    file_identity: FileIdentity,
    lost: AtomicBool,
}

impl ConfigurationStore {
    pub fn open(
        directory: PrivateDirectory,
        identity: PeerIdentity,
        seed: Option<Configuration>,
    ) -> Result<Self> {
        if let Some(seed) = &seed {
            seed.validate(identity)?;
        }
        let path = directory.path().join(RECORD_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        let object = directory.durable_identity()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (record, file_identity) = if namespace.file_is_missing(&path)? {
            let state = ConfigurationState {
                schema_version: CONFIGURATION_SCHEMA,
                identity,
                revision: FIRST_CONFIGURATION_REVISION,
                last_change: Uuid::new_v4(),
                configuration: seed.clone().unwrap_or_default(),
            };
            let record = Record {
                schema_version: PRIVATE_RECORD_SCHEMA,
                directory: object,
                state,
                _legacy_listener_binding: false,
            };
            record.validate(object, identity)?;
            let written =
                namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
                    lock.verify()
                })?;
            (record, written)
        } else {
            namespace.read_json_with_identity(&path)?
        };
        record.validate(object, identity)?;
        ensure!(
            seed.is_none_or(|seed| seed == record.state.configuration),
            SEED_CONFLICT
        );
        let store = Self {
            identity,
            directory: object,
            path,
            namespace,
            lock,
            file_identity,
            lost: AtomicBool::new(false),
        };
        store.current()?;
        Ok(store)
    }

    pub fn current(&self) -> Result<ConfigurationState> {
        let result = self.read_current();
        self.retain_failure(result)
    }

    /// True once a read or write failure has made this handle unusable.
    /// Recovery requires reopening the store; callers must not schedule
    /// activation retries through a lost handle.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    fn read_current(&self) -> Result<ConfigurationState> {
        self.verify()?;
        let (record, identity): (Record, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(identity == self.file_identity, REPLACED_RECORD);
        record.validate(self.directory, self.identity)?;
        self.verify()?;
        Ok(record.state)
    }

    pub(crate) fn write(&mut self, state: ConfigurationState) -> Result<()> {
        let record = Record {
            schema_version: PRIVATE_RECORD_SCHEMA,
            directory: self.directory,
            state,
            _legacy_listener_binding: false,
        };
        self.write_record(&record, || Ok(()))
    }

    fn write_record(
        &mut self,
        record: &Record,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        record.validate(self.directory, self.identity)?;
        let result = (|| {
            self.verify()?;
            self.namespace.write_json_expected_with_commit_fence(
                &self.path,
                Some(self.file_identity),
                record,
                || {
                    self.lock.verify()?;
                    commit_fence()
                },
            )
        })();
        self.file_identity = self.retain_failure(result)?;
        Ok(())
    }

    fn verify(&self) -> Result<()> {
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.lock.verify()?;
        self.namespace.verify()
    }

    fn retain_failure<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}
