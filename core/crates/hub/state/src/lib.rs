mod constants;
mod contract;
mod inspection;
mod peer_key;
mod record;

use anyhow::{Context, Result};
use record::{Binding, Record};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::path::{Path, PathBuf};
use yolo_hub_identity::HubId;
use yolo_paths::namespace_lease::NamespaceLease;
use yolo_paths::{coordination::CoordinationLock, directory::PrivateDirectory};

use constants::*;
pub use constants::{HUB_IDENTITY_FILE, HUB_SOCKET_FILE};

/// Read-only admission probe used by the upgrade coordinator.
pub fn validate_runtime_contract(path: &Path) -> Result<()> {
    contract::validate_path(path)
}

pub struct HubState {
    hub: HubId,
    path: PathBuf,
    key: Ed25519KeyPair,
    namespace: NamespaceLease,
    socket_namespace: yolo_paths::socket_namespace::SocketNamespace,
    socket: PathBuf,
    // Keep singleton custody until the short namespace has finished retiring.
    lock: CoordinationLock,
    _host_owner: CoordinationLock,
}

impl HubState {
    pub fn open(path: &Path, host_home: &Path) -> Result<Self> {
        // Coordination keys fold case/firmlinks; retained descriptors require the real path.
        let path = yolo_paths::platform::platform_path(path);
        let directory = PrivateDirectory::ensure(&path)?;
        let path = directory.path().to_path_buf();
        let lock = CoordinationLock::try_acquire(&path.join(HUB_LOCK_FILE), MAX_LOCK_BYTES)?
            .context(ALREADY_RUNNING)?;
        let binding = Binding::capture(&directory)?;
        let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, directory)])?;
        let host_home = yolo_paths::coordination::canonical_logical_path(host_home)?;
        let owner_path = yolo_paths::coordination::lock_path(
            yolo_paths::coordination::CoordinationScope::HostHome,
            &host_home,
            Some(HOST_COORDINATOR),
        )?;
        let host_owner =
            CoordinationLock::try_acquire(&owner_path, MAX_LOCK_BYTES)?.context(ALREADY_RUNNING)?;
        let identity_path = path.join(HUB_IDENTITY_FILE);
        let fresh = namespace.file_is_missing(&identity_path)?;
        let record = if fresh {
            let key = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
                .map_err(|_| anyhow::anyhow!(KEY_FAILURE))?;
            let record = Record {
                schema: STATE_SCHEMA_VERSION,
                hub_id: HubId::new(),
                binding,
                host_home,
                key: key.as_ref().to_vec(),
            };
            namespace.write_json(&identity_path, &record)?;
            record
        } else {
            let record: Record = namespace.read_json(&identity_path)?;
            record.verify(&binding, &host_home)?;
            record
        };
        contract::open_or_validate(&namespace, &path, fresh)?;
        let key =
            Ed25519KeyPair::from_pkcs8(&record.key).map_err(|_| anyhow::anyhow!(KEY_FAILURE))?;
        let socket_namespace = yolo_paths::Paths::new(&path).retain_socket_namespace()?;
        let socket = socket_namespace.hub_socket()?;
        let state = Self {
            hub: record.hub_id,
            path,
            key,
            namespace,
            lock,
            _host_owner: host_owner,
            socket_namespace,
            socket,
        };
        state.verify()?;
        Ok(state)
    }

    pub fn hub_id(&self) -> HubId {
        self.hub
    }
    pub fn socket_path(&self) -> PathBuf {
        self.socket.clone()
    }
    pub fn socket_path_for(state: &Path) -> Result<PathBuf> {
        yolo_paths::Paths::new(state).hub_socket()
    }
    pub fn hub_id_for(path: &Path, host_home: &Path) -> Result<HubId> {
        let directory = PrivateDirectory::open(path)?;
        let namespace = NamespaceLease::from_directories(vec![(STATE_NAMESPACE, directory)])?;
        let record: Record = namespace.read_json(&path.join(HUB_IDENTITY_FILE))?;
        let binding = Binding::capture(&namespace.directory(path)?)?;
        let host_home = yolo_paths::coordination::canonical_logical_path(host_home)?;
        record.verify(&binding, &host_home)?;
        Ok(record.hub_id)
    }
    pub fn socket_directory(&self) -> Result<PrivateDirectory> {
        self.verify()?;
        self.socket_namespace.directory()
    }
    pub fn public_key(&self) -> &[u8] {
        self.key.public_key().as_ref()
    }
    pub fn sign(&self, message: &[u8]) -> Result<ring::signature::Signature> {
        self.verify()?;
        Ok(self.key.sign(message))
    }
    pub fn directory(&self) -> Result<PrivateDirectory> {
        self.verify()?;
        self.namespace.directory(&self.path)
    }
    pub fn verify(&self) -> Result<()> {
        self.lock.verify()?;
        self._host_owner.verify()?;
        self.namespace.verify()?;
        self.socket_namespace.verify()
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
