use anyhow::Result;
use std::path::PathBuf;
use yolo_runtime::paths::{Paths, directory::PrivateDirectory, socket_namespace::SocketNamespace};

/// Startup admission stays separate from validation-only endpoint discovery.
pub(crate) struct HostNamespace {
    home: PrivateDirectory,
    namespace: SocketNamespace,
}

impl HostNamespace {
    pub(crate) fn retain(paths: &Paths) -> Result<Self> {
        let home = PrivateDirectory::open_or_create_strict(paths.home())?;
        let namespace = paths.retain_socket_namespace()?;
        let retained = Self { home, namespace };
        retained.verify()?;
        Ok(retained)
    }

    pub(crate) fn socket(&self) -> Result<PathBuf> {
        self.verify()?;
        self.namespace.host_socket()
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.home.verify_path()?;
        self.namespace.verify()
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/host_namespace.rs"]
mod tests;
