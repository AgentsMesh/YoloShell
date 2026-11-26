use crate::{constants::*, record::Record};
use anyhow::{Context, Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_hub_client_access_protocol::AccessConfiguration;
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock,
    identity::{DurableObjectIdentity, FileIdentity},
    namespace_lease::NamespaceLease,
};

pub struct ClientAccessStore {
    pub(crate) entry_hub: HubId,
    pub(crate) directory: DurableObjectIdentity,
    pub(crate) path: PathBuf,
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) file_identity: FileIdentity,
    pub(crate) configuration: AccessConfiguration,
    pub(crate) lost: AtomicBool,
}

impl ClientAccessStore {
    pub fn current(&self) -> Result<AccessConfiguration> {
        self.retain_failure(self.read_current())
    }

    /// A successful result is the exact committed configuration. Any failure
    /// requires revoking live admissions; this instance never resumes writes.
    pub fn replace(
        &mut self,
        expected_revision: u64,
        next: AccessConfiguration,
    ) -> Result<AccessConfiguration> {
        let result = (|| {
            let current = self.read_current()?;
            next.validate()?;
            ensure!(next.entry_hub == self.entry_hub, WRONG_IDENTITY);
            ensure!(current.revision == expected_revision, REVISION_CONFLICT);
            let revision = expected_revision
                .checked_add(REVISION_STEP)
                .context(REVISION_EXHAUSTED)?;
            ensure!(next.revision == revision, REVISION_CONFLICT);
            let record = Record {
                schema_version: PRIVATE_RECORD_SCHEMA,
                directory: self.directory,
                configuration: next.clone(),
            };
            record.validate(self.directory, self.entry_hub)?;
            let identity = self.namespace.write_json_expected_with_commit_fence(
                &self.path,
                Some(self.file_identity),
                &record,
                || self.read_current().map(|_| ()),
            )?;
            self.file_identity = identity;
            self.configuration = next;
            self.read_current()
        })();
        self.retain_failure(result)
    }

    fn read_current(&self) -> Result<AccessConfiguration> {
        self.verify()?;
        let (record, identity): (Record, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(
            identity == self.file_identity && record.configuration == self.configuration,
            REPLACED_RECORD
        );
        record.validate(self.directory, self.entry_hub)?;
        self.verify()?;
        Ok(record.configuration)
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
