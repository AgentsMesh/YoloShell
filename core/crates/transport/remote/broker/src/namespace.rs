use super::broker_paths::BrokerPaths;
use super::constants::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result, ensure};
use std::path::Path;

#[derive(Debug)]
pub(super) struct BrokerNamespace {
    lease: NamespaceLease,
    state: PrivateDirectory,
}

impl BrokerNamespace {
    pub(super) fn capture(paths: &BrokerPaths) -> Result<Self> {
        let runtime = paths.runtime_socket();
        let management = paths.management_socket();
        let runtime_parent = socket_parent(&runtime)?;
        let management_parent = socket_parent(&management)?;
        let state = paths.state_directory()?;
        ensure!(
            runtime_parent == state.path() && management_parent == state.path(),
            BROKER_STATE_ANCHOR_MISMATCH
        );
        state.verify_path()?;
        let lease = NamespaceLease::from_directories(vec![
            (BROKER_STATE_NAMESPACE_LABEL, state.try_clone()?),
            (BROKER_RUNTIME_PARENT_NAMESPACE_LABEL, state.try_clone()?),
            (BROKER_MANAGEMENT_PARENT_NAMESPACE_LABEL, state.try_clone()?),
        ])?;
        state.verify_path()?;
        Ok(Self { lease, state })
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.lease.verify().context(BROKER_NAMESPACE_LOST_CONTEXT)
    }

    pub(super) async fn watch(&self) -> Result<std::convert::Infallible> {
        loop {
            tokio::time::sleep(BROKER_NAMESPACE_POLL_INTERVAL).await;
            self.verify()?;
        }
    }

    pub(super) fn state_directory(&self) -> Result<PrivateDirectory> {
        self.state.try_clone()
    }
}

fn socket_parent(path: &Path) -> Result<&Path> {
    path.parent().context(BROKER_SOCKET_PARENT_ERROR)
}
