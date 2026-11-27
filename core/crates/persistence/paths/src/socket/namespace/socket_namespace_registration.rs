use super::NamespaceBinding;
use crate::paths::{
    identity::{DurableObjectIdentity, FileIdentity, ObjectIdentity},
    private_file::PrivateFileEntry,
    process_file_lock::{Mode, ProcessFileLock},
};
use crate::{coordination::CoordinationLock, namespace_lease::NamespaceLease};
use anyhow::{Context, Result, ensure};
use std::path::Path;
#[path = "registration/constants.rs"]
mod constants;
#[path = "registration/read.rs"]
mod read;
#[path = "registration/record.rs"]
mod record;
use constants::*;

#[derive(Debug)]
pub(super) struct Registration {
    entry: PrivateFileEntry,
    identity: FileIdentity,
    durable_identity: DurableObjectIdentity,
    owner_pid: u32,
    lock: Option<ProcessFileLock>,
}

impl Registration {
    pub(super) fn try_existing(
        home: &Path,
        expected: &NamespaceBinding,
        mode: Mode,
    ) -> Result<Option<Self>> {
        let Some((directory, entry, mut opened)) = read::open_existing(home)? else {
            return Ok(None);
        };
        let Some(lock) =
            ProcessFileLock::try_lock(opened.file(), opened.identity().object_identity(), mode)?
        else {
            return Ok(None);
        };
        read::verify_opened(&directory, &entry, &mut opened, expected)?;
        let durable_identity = read::durable_identity(&entry, &opened)?;
        Ok(Some(Self {
            entry,
            identity: opened.identity(),
            durable_identity,
            owner_pid: std::process::id(),
            lock: Some(lock),
        }))
    }

    pub(super) fn identity(&self) -> ObjectIdentity {
        self.identity.object_identity()
    }

    pub(super) fn durable_identity(&self) -> DurableObjectIdentity {
        self.durable_identity
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure!(self.lock.is_some(), REGISTRATION_BUSY);
        self.verify_entry()
    }

    /// The caller holds namespace lifecycle serialization while relinquishing its shared lease.
    /// Failure leaves no ownership grant; another immutable registration is never adopted.
    pub(super) fn try_upgrade(&mut self) -> Result<bool> {
        self.verify_entry()?;
        self.lock.take();
        let opened = self.entry.open_read(REGISTRATION_MAX_BYTES)?;
        ensure!(opened.identity() == self.identity, INVALID_REGISTRATION);
        let Some(lock) = ProcessFileLock::try_lock(
            opened.file(),
            self.identity.object_identity(),
            Mode::Exclusive,
        )?
        else {
            return Ok(false);
        };
        self.lock = Some(lock);
        self.verify()?;
        Ok(true)
    }

    fn verify_entry(&self) -> Result<()> {
        ensure!(self.owner_pid == std::process::id(), PROCESS_CHANGED);
        self.entry
            .verify_current_unchanged(self.identity, REGISTRATION_MAX_BYTES)
    }
}

pub(super) fn retain(
    home: &Path,
    expected: &NamespaceBinding,
    create_allowed: bool,
) -> Result<Registration> {
    if let Some(registration) = Registration::try_existing(home, expected, Mode::Shared)? {
        return Ok(registration);
    }
    let (directory, entry) = read::entry(home)?;
    if !entry.is_missing()? {
        anyhow::bail!(REGISTRATION_BUSY);
    }
    ensure!(create_allowed, REGISTRATION_MISSING);
    let initialization = CoordinationLock::try_acquire(
        &directory.path().join(INITIALIZATION_LOCK),
        REGISTRATION_MAX_BYTES,
    )?
    .context(INITIALIZATION_BUSY)?;
    if entry.is_missing()? {
        let path = entry.path().to_path_buf();
        let record = record::Record::new(directory.durable_identity()?, expected)?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        namespace.write_json_expected_with_commit_fence(&path, None, &record, || {
            initialization.verify()
        })?;
    }
    Registration::try_existing(home, expected, Mode::Shared)?.context(REGISTRATION_BUSY)
}

/// Bounded read-only identity lookup. Never creates a registration or coordination file.
pub(super) fn inspect(
    home: &Path,
    expected: &NamespaceBinding,
) -> Result<Option<DurableObjectIdentity>> {
    let Some((directory, entry, mut opened)) = read::open_existing(home)? else {
        return Ok(None);
    };
    read::verify_opened(&directory, &entry, &mut opened, expected)?;
    Ok(Some(read::durable_identity(&entry, &opened)?))
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_registration_durable_tests.rs"]
mod durable_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_registration_tests.rs"]
mod tests;
