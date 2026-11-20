use super::constants::*;
use super::factory::RusshConnectionFactory;
use super::handlers::{close_saturated, manage_upgradable, setup_runtime_admitted};
use super::maintenance_runtime::BrokerMaintenance;
use super::namespace::BrokerNamespace;
use super::server_admission::BrokerAdmission;
use super::sockets::BrokerSockets;
use super::{
    AuthorizedBrokerStartup, BrokerConfig, BrokerConnectionFactory, BrokerConnectionPool,
    BrokerLaunchAuthority, BrokerPaths,
};
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use futures::FutureExt;
use std::future::Future;
use std::io;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Notify, Semaphore};

pub struct RemoteBroker {
    config: Arc<BrokerConfig>,
    paths: BrokerPaths,
    pool: Arc<BrokerConnectionPool>,
    shutdown: Arc<Notify>,
    retirement: Arc<Notify>,
    launch_authority: Option<BrokerLaunchAuthority>,
    startup_lock: Option<super::instance_lock::BrokerInstanceLock>,
    startup_deadline: Option<tokio::time::Instant>,
}

impl RemoteBroker {
    pub fn new(config: BrokerConfig, paths: BrokerPaths) -> Self {
        let factory = Arc::new(RusshConnectionFactory::new(config.clone()));
        Self::with_factory(config, paths, factory)
    }

    pub fn new_authorized(config: BrokerConfig, startup: AuthorizedBrokerStartup) -> Self {
        let factory = Arc::new(RusshConnectionFactory::new(config.clone()));
        Self::with_authorized_factory(config, startup, factory)
    }

    pub fn with_factory(
        config: BrokerConfig,
        paths: BrokerPaths,
        factory: Arc<dyn BrokerConnectionFactory>,
    ) -> Self {
        Self::finish(config, paths, factory, None)
    }

    fn finish(
        config: BrokerConfig,
        paths: BrokerPaths,
        factory: Arc<dyn BrokerConnectionFactory>,
        startup: Option<(
            BrokerLaunchAuthority,
            super::instance_lock::BrokerInstanceLock,
            tokio::time::Instant,
        )>,
    ) -> Self {
        let mut pool = BrokerConnectionPool::configured(&config, factory);
        pool.retain_provisioning_paths(&paths);
        let (launch_authority, startup_lock, startup_deadline) = match startup {
            Some((authority, lock, deadline)) => (Some(authority), Some(lock), Some(deadline)),
            None => (None, None, None),
        };
        Self {
            config: Arc::new(config),
            paths,
            pool: Arc::new(pool),
            shutdown: Arc::new(Notify::new()),
            retirement: Arc::new(Notify::new()),
            launch_authority,
            startup_lock,
            startup_deadline,
        }
    }

    pub fn paths(&self) -> &BrokerPaths {
        &self.paths
    }
    pub fn pool(&self) -> &Arc<BrokerConnectionPool> {
        &self.pool
    }

    pub async fn serve(self) -> Result<()> {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        self.serve_until(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
        })
        .await
    }

    pub async fn serve_until<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        self.serve_until_with_role_limits(
            shutdown,
            None,
            MAX_RUNTIME_CLIENTS,
            MAX_RUNTIME_ATTACHMENTS,
            MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn serve_until_with_limits<F>(
        self,
        shutdown: F,
        runtime_limit: usize,
        management_limit: usize,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        let maintenance = BrokerMaintenance::disabled();
        self.run_server(
            shutdown,
            None,
            runtime_limit,
            runtime_limit,
            management_limit,
            maintenance,
        )
        .await
    }

    async fn serve_until_with_role_limits<F>(
        self,
        shutdown: F,
        adoption: Option<tokio::sync::oneshot::Receiver<super::BrokerAdoptionAcknowledgement>>,
        runtime_limit: usize,
        attachment_limit: usize,
        management_limit: usize,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        let maintenance = BrokerMaintenance::runtime()?;
        self.run_server(
            shutdown,
            adoption,
            runtime_limit,
            attachment_limit,
            management_limit,
            maintenance,
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn serve_until_with_maintenance<F>(
        self,
        shutdown: F,
        maintenance: BrokerMaintenance,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        self.run_server(
            shutdown,
            None,
            MAX_RUNTIME_CLIENTS,
            MAX_RUNTIME_ATTACHMENTS,
            MAX_MANAGEMENT_CLIENTS,
            maintenance,
        )
        .await
    }
}

include!("server_factory.rs");
include!("server_hub_tasks.rs");

include!("server_tasks.rs");
include!("server_supervised.rs");
include!("server_shutdown.rs");
include!("server_task_drain.rs");
include!("server_task_group.rs");
include!("server_lifecycle.rs");
include!("server_listener.rs");

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/server/server_listener_instance_matrix.rs"]
mod server_listener_instance_matrix;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/server/server_task_drain_cancellation_tests.rs"]
mod server_task_drain_cancellation_tests;
