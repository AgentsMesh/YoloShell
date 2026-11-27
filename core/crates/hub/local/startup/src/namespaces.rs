use crate::{Launch, fence::StartupFence};
use anyhow::Result;
use std::path::PathBuf;
use yolo_paths::{Paths, directory::PrivateDirectory, socket_namespace::SocketNamespace};

/// Admission is a startup operation. Endpoint lookup remains validation-only.
/// Retain both namespaces until committed services have acquired their own custody.
pub(crate) struct StartupNamespaces {
    host_home: PrivateDirectory,
    hub: SocketNamespace,
    host: SocketNamespace,
}

impl StartupNamespaces {
    pub(crate) fn retain(launch: &Launch, fence: &StartupFence) -> Result<Self> {
        fence.verify()?;
        let host_home = PrivateDirectory::open_or_create_strict(&launch.host_home)?;
        let hub = Paths::new(&launch.state).retain_socket_namespace()?;
        let host = Paths::new(&launch.host_home).retain_socket_namespace()?;
        let retained = Self {
            host_home,
            hub,
            host,
        };
        retained.verify()?;
        fence.verify()?;
        Ok(retained)
    }

    pub(crate) fn hub_socket(&self) -> Result<PathBuf> {
        self.verify()?;
        self.hub.hub_socket()
    }

    pub(crate) fn host_socket(&self) -> Result<PathBuf> {
        self.verify()?;
        self.host.host_socket()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.host_home.verify_path()?;
        self.hub.verify()?;
        self.host.verify()
    }
}
