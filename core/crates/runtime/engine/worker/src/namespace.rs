use crate::namespace_lease::{NamespaceLease, NamespaceOwnershipLoss, NamespaceVerification};
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const HOME_LABEL: &str = "worker home";
const SESSION_LABEL: &str = "worker session";
const TERMINAL_LABEL: &str = "worker terminal";
const EVENTS_LABEL: &str = "worker events";
const SOCKET_LABEL: &str = "worker socket parent";

#[derive(Debug)]
pub(super) struct WorkerNamespace {
    lease: Arc<NamespaceLease>,
    socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
}

impl WorkerNamespace {
    pub(super) fn capture(paths: &Paths, terminal_id: Uuid) -> Result<Self> {
        let session = paths.session_dir(terminal_id);
        let terminal = paths.terminal_dir(terminal_id);
        let events = paths.events_dir(terminal_id);
        let socket_namespace = Arc::new(paths.retain_socket_namespace()?);
        let socket = socket_namespace.worker_socket(terminal_id)?;
        let socket_parent = socket.parent().context("worker socket has no parent")?;
        let lease = Arc::new(NamespaceLease::capture(&[
            (HOME_LABEL, paths.home()),
            (SESSION_LABEL, &session),
            (TERMINAL_LABEL, &terminal),
            (EVENTS_LABEL, &events),
            (SOCKET_LABEL, socket_parent),
        ])?);
        Ok(Self {
            lease,
            socket_namespace,
        })
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.verification().into_result()
    }

    pub(super) fn verification(&self) -> NamespaceVerification {
        match self.lease.verification() {
            NamespaceVerification::Verified => match self.socket_namespace.verify() {
                Ok(()) => NamespaceVerification::Verified,
                Err(error) => NamespaceVerification::Indeterminate(error),
            },
            settled => settled,
        }
    }

    pub(super) fn prove_ownership_loss(&self) -> Option<NamespaceOwnershipLoss> {
        self.lease.prove_ownership_loss()
    }

    pub(super) fn lease(&self) -> &NamespaceLease {
        &self.lease
    }

    pub(super) fn lease_arc(&self) -> Arc<NamespaceLease> {
        Arc::clone(&self.lease)
    }

    pub(super) fn socket_namespace(&self) -> Arc<crate::paths::socket_namespace::SocketNamespace> {
        Arc::clone(&self.socket_namespace)
    }

    pub(super) fn worker_socket(&self, terminal_id: Uuid) -> Result<std::path::PathBuf> {
        self.socket_namespace.worker_socket(terminal_id)
    }

    pub(super) fn claude_hook_socket(&self, terminal_id: Uuid) -> Result<std::path::PathBuf> {
        self.socket_namespace.claude_hook_socket(terminal_id)
    }

    pub(super) fn codex_hook_socket(&self, terminal_id: Uuid) -> Result<std::path::PathBuf> {
        self.socket_namespace.codex_hook_socket(terminal_id)
    }

    pub(super) fn qwen_hook_socket(&self, terminal_id: Uuid) -> Result<std::path::PathBuf> {
        self.socket_namespace.qwen_hook_socket(terminal_id)
    }

    pub(super) fn provider_hook_socket(&self, terminal_id: Uuid) -> Result<std::path::PathBuf> {
        self.socket_namespace.provider_hook_socket(terminal_id)
    }

    pub(super) fn read_json<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        self.lease.read_json(path)
    }

    pub(super) fn read_json_with_identity<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<(T, FileIdentity)> {
        self.lease.read_json_with_identity(path)
    }

    pub(super) fn write_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        self.lease.write_json(path, value)
    }

    pub(super) fn file(&self, path: &Path) -> Result<crate::paths::private_file::PrivateFileEntry> {
        self.lease.file(path)
    }

    pub(super) fn ensure_directory(&self, path: &Path) -> Result<()> {
        self.lease.ensure_directory(path).map(|_| ())
    }

    pub(super) fn remove_file(&self, path: &Path) -> Result<()> {
        self.lease.remove_file(path)?;
        #[cfg(test)]
        super::runtime_persistence_test_hook::record_removal(path)?;
        Ok(())
    }

    pub(super) fn file_is_missing(&self, path: &Path) -> Result<bool> {
        self.lease.file_is_missing(path)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/namespace/namespace_tests.rs"]
mod tests;
