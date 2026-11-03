use super::broker_paths::BrokerPaths;
use super::client::{BrokerClient, BrokerTransition};
use super::constants::*;
use super::handoff_endpoint::{retire_endpoint, socket_name};
use super::instance_lock::BrokerInstanceLock;
use super::socket_directory::SocketDirectory;
use super::socket_identity::SocketIdentity;
use anyhow::{Context, Result};
use std::ffi::CString;

#[path = "handoff_guard.rs"]
mod reconcile_guard;
pub use reconcile_guard::BrokerReconcileGuard;
#[path = "handoff_hub.rs"]
mod hub;
#[cfg(test)]
use reconcile_guard::acquire_reconcile_with_hook;

pub struct BrokerRetirement {
    directory: SocketDirectory,
    runtime_name: CString,
    management_name: CString,
    runtime: Option<SocketIdentity>,
    management: Option<SocketIdentity>,
}

impl BrokerRetirement {
    pub fn capture(guard: &BrokerReconcileGuard) -> Result<Self> {
        guard.verify()?;
        let directory = SocketDirectory::new(guard.state_directory()?);
        let runtime_name = socket_name(RUNTIME_SOCKET_FILE)?;
        let management_name = socket_name(MANAGEMENT_SOCKET_FILE)?;
        let value = Self {
            runtime: directory.identity(&runtime_name)?,
            management: directory.identity(&management_name)?,
            directory,
            runtime_name,
            management_name,
        };
        guard.verify()?;
        Ok(value)
    }

    pub async fn transition(
        &self,
        client: &BrokerClient,
        desired_revision: &str,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerTransition> {
        guard.verify()?;
        let identity = self
            .management
            .context(BROKER_MANAGEMENT_ENDPOINT_MISSING)?;
        let transition = client
            .transition_at(desired_revision, identity, deadline)
            .await;
        guard.verify()?;
        Ok(transition?)
    }

    pub async fn bootstrap(
        &self,
        client: &BrokerClient,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
    ) -> Result<super::BrokerStatus> {
        guard.verify()?;
        let identity = self
            .management
            .context(BROKER_MANAGEMENT_ENDPOINT_MISSING)?;
        let status = client.bootstrap_at(identity, deadline).await;
        guard.verify()?;
        Ok(status?)
    }

    pub(super) fn has_management_endpoint(&self) -> bool {
        self.management.is_some()
    }

    pub(super) async fn request_retirement(
        &self,
        client: &BrokerClient,
        expected_host_id: &str,
        guard: &BrokerReconcileGuard,
        deadline: tokio::time::Instant,
    ) -> Result<()> {
        guard.verify()?;
        let identity = self
            .management
            .context(BROKER_MANAGEMENT_ENDPOINT_MISSING)?;
        let result = client.retire_at(expected_host_id, identity, deadline).await;
        guard.verify()?;
        Ok(result?)
    }

    pub async fn retire_if_unlocked(
        &self,
        paths: &BrokerPaths,
        guard: &BrokerReconcileGuard,
    ) -> Result<bool> {
        guard.verify()?;
        let ownership = BrokerInstanceLock::try_acquire(paths.instance_lock())?;
        guard.verify()?;
        let Some(_ownership) = ownership else {
            return Ok(false);
        };
        let runtime = retire_endpoint(
            &self.directory,
            &self.runtime_name,
            &paths.runtime_socket(),
            self.runtime,
        )
        .await?;
        guard.verify()?;
        if !runtime {
            return Ok(false);
        }
        let management = retire_endpoint(
            &self.directory,
            &self.management_name,
            &paths.management_socket(),
            self.management,
        )
        .await?;
        guard.verify()?;
        Ok(management)
    }
}

pub fn broker_instance_available(paths: &BrokerPaths) -> Result<bool> {
    BrokerInstanceLock::try_acquire(paths.instance_lock()).map(|lock| lock.is_some())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/handoff/handoff_private_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/handoff/handoff_lock_tests.rs"]
mod lock_tests;
