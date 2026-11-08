use super::broker_paths::BrokerPaths;
use super::constants::BROKER_PEER_OWNER_ERROR;
use super::instance_lock::BrokerInstanceLock;
use super::launch_authority::BrokerLaunchAuthority;
use super::namespace::BrokerNamespace;
use super::socket_binding::{
    SocketCleanup, bind_private, prepare_endpoint as prepare_retained_endpoint,
};
use super::socket_directory::SocketDirectory;
use super::socket_identity::socket_identity;
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
#[path = "sockets_authorized.rs"]
mod authorized;

const BROKER_PEER_CREDENTIAL_CONTEXT: &str = "read broker peer credentials";

pub(super) struct BrokerSockets {
    pub runtime: UnixListener,
    pub management: UnixListener,
    _cleanup: [SocketCleanup; 2],
    _lock: BrokerInstanceLock,
    namespace: Arc<BrokerNamespace>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BrokerSocketBindStage {
    RuntimePublished,
    ManagementPublished,
}

impl BrokerSockets {
    #[cfg(test)]
    pub async fn bind(paths: &BrokerPaths) -> Result<Self> {
        let lock = BrokerInstanceLock::acquire(paths.instance_lock())?;
        Self::bind_locked(paths, lock, || Ok(()), |_| Ok(()), |_| Ok(())).await
    }

    #[cfg(test)]
    pub(super) async fn bind_with_lock_hook<F>(paths: &BrokerPaths, hook: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        let lock = BrokerInstanceLock::acquire_with_hook(paths.instance_lock(), hook)?;
        Self::bind_locked(paths, lock, || Ok(()), |_| Ok(()), |_| Ok(())).await
    }

    #[cfg(test)]
    pub(super) async fn bind_with_namespace_hook<F>(paths: &BrokerPaths, hook: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        let lock = BrokerInstanceLock::acquire(paths.instance_lock())?;
        Self::bind_locked(paths, lock, hook, |_| Ok(()), |_| Ok(())).await
    }

    #[cfg(test)]
    pub(super) async fn bind_with_socket_hook<F>(paths: &BrokerPaths, hook: F) -> Result<Self>
    where
        F: FnMut(BrokerSocketBindStage) -> Result<()>,
    {
        let lock = BrokerInstanceLock::acquire(paths.instance_lock())?;
        Self::bind_locked(paths, lock, || Ok(()), |_| Ok(()), hook).await
    }

    async fn bind_locked<N, B, S>(
        paths: &BrokerPaths,
        lock: BrokerInstanceLock,
        namespace_hook: N,
        mut before_publish: B,
        mut socket_hook: S,
    ) -> Result<Self>
    where
        N: FnOnce() -> Result<()>,
        B: FnMut(BrokerSocketBindStage) -> Result<()>,
        S: FnMut(BrokerSocketBindStage) -> Result<()>,
    {
        let namespace = Arc::new(BrokerNamespace::capture(paths)?);
        namespace_hook()?;
        namespace.verify()?;
        let runtime_path = paths.runtime_socket();
        let management_path = paths.management_socket();
        let runtime_name = endpoint_name(&runtime_path)?;
        let management_name = endpoint_name(&management_path)?;
        let state = SocketDirectory::new(namespace.state_directory()?);
        prepare_retained_endpoint(&state, &runtime_name)?;
        namespace.verify()?;
        prepare_retained_endpoint(&state, &management_name)?;
        namespace.verify()?;
        before_publish(BrokerSocketBindStage::RuntimePublished)?;
        let runtime = bind_private(&state, runtime_name)?;
        socket_hook(BrokerSocketBindStage::RuntimePublished)?;
        namespace.verify()?;
        before_publish(BrokerSocketBindStage::ManagementPublished)?;
        let management = bind_private(&state, management_name)?;
        socket_hook(BrokerSocketBindStage::ManagementPublished)?;
        namespace.verify()?;
        Ok(Self {
            runtime: runtime.listener,
            management: management.listener,
            _cleanup: [runtime.cleanup, management.cleanup],
            _lock: lock,
            namespace,
        })
    }

    pub(super) fn namespace(&self) -> Arc<BrokerNamespace> {
        Arc::clone(&self.namespace)
    }

    pub(super) fn close_for_retirement(self) -> BrokerInstanceLock {
        let Self {
            runtime,
            management,
            _cleanup,
            _lock,
            namespace,
        } = self;
        drop(runtime);
        drop(management);
        drop(_cleanup);
        drop(namespace);
        _lock
    }

    #[cfg(test)]
    pub(super) fn verify_namespace(&self) -> Result<()> {
        self.namespace.verify()
    }
}

pub(super) trait BrokerPeerCredentials {
    fn broker_peer_credentials(&self) -> std::io::Result<tokio::net::unix::UCred>;
}

impl BrokerPeerCredentials for UnixStream {
    fn broker_peer_credentials(&self) -> std::io::Result<tokio::net::unix::UCred> {
        self.peer_cred()
    }
}

impl BrokerPeerCredentials for LocalUnixStream {
    fn broker_peer_credentials(&self) -> std::io::Result<tokio::net::unix::UCred> {
        self.peer_cred()
    }
}

pub(super) fn verify_peer(stream: &impl BrokerPeerCredentials) -> Result<()> {
    let peer = stream
        .broker_peer_credentials()
        .context(BROKER_PEER_CREDENTIAL_CONTEXT)?;
    // SAFETY: geteuid has no preconditions and only reads credentials.
    let expected = unsafe { libc::geteuid() };
    ensure!(peer.uid() == expected, BROKER_PEER_OWNER_ERROR);
    Ok(())
}

pub fn validate_broker_socket(path: &Path) -> Result<()> {
    validate_broker_socket_path(path)?;
    socket_identity(path)
        .map(|_| ())
        .context("validate broker socket")
}

pub fn validate_broker_socket_path(path: &Path) -> Result<()> {
    super::security::validate_socket_path(path)
}

fn endpoint_name(path: &Path) -> Result<CString> {
    let name = path.file_name().context("broker socket path has no name")?;
    CString::new(name.as_bytes()).context("broker socket name contains a null byte")
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/sockets_failure_tests.rs"]
mod failure_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/sockets_instance_matrix.rs"]
mod instance_matrix;
