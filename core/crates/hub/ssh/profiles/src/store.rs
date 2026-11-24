use crate::{
    SshBrokerProfile, SshProfileConfiguration, SshProfileState, constants::*, record::Record,
};
use anyhow::{Context, Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_paths::{
    coordination::CoordinationLock, identity::FileIdentity, namespace_lease::NamespaceLease,
};
use yolo_ssh_profile_protocol::SshProfileRef;

/// One registry owner. Storage or identity failures poison this instance; resolving an
/// unknown profile and rejected optimistic revisions do not invalidate healthy state.
pub struct SshProfileStore {
    pub(crate) record: Record,
    pub(crate) path: PathBuf,
    pub(crate) namespace: NamespaceLease,
    pub(crate) lock: CoordinationLock,
    pub(crate) file: FileIdentity,
    pub(crate) lost: AtomicBool,
}

impl SshProfileStore {
    pub fn current(&self) -> Result<SshProfileState> {
        self.retain_failure(self.read_current())
    }
    pub fn verify(&self) -> Result<()> {
        self.current().map(|_| ())
    }
    pub fn configuration(&self) -> Result<SshProfileConfiguration> {
        Ok(self.current()?.configuration)
    }
    pub fn resolve(&self, profile: &SshProfileRef) -> Result<SshBrokerProfile> {
        self.current()?
            .configuration
            .profiles
            .into_iter()
            .find(|entry| &entry.profile == profile)
            .context(UNKNOWN_PROFILE)
    }
    pub(crate) fn read_current(&self) -> Result<SshProfileState> {
        self.verify_namespace()?;
        let (record, file) = self
            .namespace
            .read_json_with_identity::<Record>(&self.path)?;
        ensure!(file == self.file && record == self.record, IDENTITY_CHANGED);
        record.validate(self.record.directory, self.record.state.hub)?;
        self.verify_namespace()?;
        Ok(record.state)
    }
    pub(crate) fn verify_namespace(&self) -> Result<()> {
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.namespace.verify()?;
        self.lock.verify()
    }
    pub(crate) fn retain_failure<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}
