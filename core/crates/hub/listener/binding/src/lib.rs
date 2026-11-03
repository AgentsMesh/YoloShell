mod constants;
mod model;
mod update;

use anyhow::{Context, Result, ensure};
pub use constants::LISTENER_BINDING_FILE;
use constants::*;
pub use model::persistent;
use model::{Entry, Record};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::ConfigurationState;
use yolo_paths::{
    coordination::CoordinationLock,
    directory::PrivateDirectory,
    identity::{DurableObjectIdentity, FileIdentity},
    namespace_lease::NamespaceLease,
};

/// Two durable slots bridge configuration CAS without changing the Configure document.
pub struct ListenerBindings {
    namespace: NamespaceLease,
    path: PathBuf,
    root: DurableObjectIdentity,
    peer: PeerIdentity,
    lock: CoordinationLock,
    file: FileIdentity,
    lost: AtomicBool,
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

impl ListenerBindings {
    pub fn open(
        directory: PrivateDirectory,
        peer: PeerIdentity,
        configuration: &ConfigurationState,
        initializing: bool,
    ) -> Result<Self> {
        configuration.validate()?;
        ensure!(configuration.identity == peer, INVALID);
        let root = directory.durable_identity()?;
        let path = directory.path().join(LISTENER_BINDING_FILE);
        let lock = CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
            .context(BUSY)?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let file = if namespace.file_is_missing(&path)? {
            ensure!(
                initializing || !persistent(configuration.configuration.listen),
                MISSING
            );
            let record = Record {
                schema: SCHEMA,
                root,
                peer,
                current: Entry {
                    configuration: configuration.clone(),
                    address: None,
                },
                pending: None,
            };
            record.validate(root, peer)?;
            namespace
                .write_json_expected_with_commit_fence(&path, None, &record, || lock.verify())?
        } else {
            let (record, file): (Record, FileIdentity) =
                namespace.read_json_with_identity(&path)?;
            record.validate(root, peer)?;
            record.selected(configuration)?;
            file
        };
        let bindings = Self {
            namespace,
            path,
            root,
            peer,
            lock,
            file,
            lost: AtomicBool::new(false),
        };
        bindings.address(configuration)?;
        Ok(bindings)
    }

    /// None for an unassigned first listener, or for an explicitly configured endpoint.
    pub fn address(&self, configuration: &ConfigurationState) -> Result<Option<SocketAddr>> {
        self.retain((|| Ok(self.read()?.selected(configuration)?.address))())
    }

    pub fn verify(&self, configuration: &ConfigurationState) -> Result<()> {
        self.retain((|| self.read()?.selected(configuration)?.ready())())
    }

    fn read(&self) -> Result<Record> {
        self.authority()?;
        let (record, file): (Record, FileIdentity) =
            self.namespace.read_json_with_identity(&self.path)?;
        ensure!(file == self.file, CHANGED);
        record.validate(self.root, self.peer)?;
        self.authority()?;
        Ok(record)
    }

    fn authority(&self) -> Result<()> {
        ensure!(!self.lost.load(Ordering::Acquire), LOST);
        self.lock.verify()?;
        self.namespace.verify()
    }

    fn retain<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
}
