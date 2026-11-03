use super::config::BrokerConfig;
use super::{BrokerConnectFuture, BrokerConnectionFactory};
use crate::remote::{
    MultiplexedSshConnection, RemoteCleanupRegistry, RemoteFuture, RemoteRuntimeSpec,
    RuntimeArtifact, RuntimeInstaller, RusshConnection, SshConnectConfig,
};
use std::sync::Arc;

pub(super) const RUNTIME_INSTALLATION_LOSS_CONTEXT: &str =
    "runtime deployment transport was interrupted";
pub(super) const RUNTIME_INSTALLATION_ERROR_MAX_CHARACTERS: usize = 512;

pub(super) trait FactoryBackend: Send + Sync {
    fn connect(
        &self,
        config: SshConnectConfig,
    ) -> RemoteFuture<'_, Arc<dyn MultiplexedSshConnection>>;

    fn install<'a>(
        &'a self,
        connection: &'a dyn MultiplexedSshConnection,
        runtime: &'a RemoteRuntimeSpec,
        artifact: &'a RuntimeArtifact,
    ) -> RemoteFuture<'a, ()>;
}

pub(super) struct LiveFactoryBackend;

impl FactoryBackend for LiveFactoryBackend {
    fn connect(
        &self,
        config: SshConnectConfig,
    ) -> RemoteFuture<'_, Arc<dyn MultiplexedSshConnection>> {
        Box::pin(async move {
            Ok(Arc::new(RusshConnection::connect(config).await?)
                as Arc<dyn MultiplexedSshConnection>)
        })
    }

    fn install<'a>(
        &'a self,
        connection: &'a dyn MultiplexedSshConnection,
        runtime: &'a RemoteRuntimeSpec,
        artifact: &'a RuntimeArtifact,
    ) -> RemoteFuture<'a, ()> {
        Box::pin(async move {
            RuntimeInstaller
                .ensure(connection, runtime, artifact)
                .await
                .map(|_| ())
        })
    }
}

pub(super) struct RusshConnectionFactory {
    config: BrokerConfig,
    backend: Arc<dyn FactoryBackend>,
}

impl RusshConnectionFactory {
    pub(super) fn new(config: BrokerConfig) -> Self {
        Self::with_backend(config, Arc::new(LiveFactoryBackend))
    }

    pub(super) fn with_backend(config: BrokerConfig, backend: Arc<dyn FactoryBackend>) -> Self {
        Self { config, backend }
    }
}

impl BrokerConnectionFactory for RusshConnectionFactory {
    fn connect_with_cleanup(&self, cleanup: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            if self.config.hub_transport && self.config.artifact.is_some() {
                return Err(crate::remote::RemoteError::InvalidRuntime(
                    yolo_broker_hub_protocol::BrokerHubError::Artifact.to_string(),
                ));
            }
            let mut ssh = SshConnectConfig::new(
                self.config.endpoint.clone(),
                self.config.known_hosts.clone(),
            )
            .use_agent(self.config.agent_socket.is_some())
            .with_cleanup_registry(cleanup);
            for identity in &self.config.identities {
                ssh = ssh.with_identity(identity.clone());
            }
            if let Some(path) = &self.config.agent_socket {
                ssh = ssh.with_agent_socket(path);
            }
            let connection = self.backend.connect(ssh).await?;
            if let Some(artifact) = &self.config.artifact {
                self.backend
                    .install(connection.as_ref(), &self.config.runtime, artifact)
                    .await
                    .map_err(classify_installation_error)?;
            }
            Ok(connection)
        })
    }
}

pub(super) fn classify_installation_error(
    error: crate::remote::RemoteError,
) -> crate::remote::RemoteError {
    if !error.is_connection_loss() {
        return error;
    }
    let detail: String = error
        .to_string()
        .chars()
        .take(RUNTIME_INSTALLATION_ERROR_MAX_CHARACTERS)
        .collect();
    crate::remote::RemoteError::RuntimeDeployment(format!(
        "{RUNTIME_INSTALLATION_LOSS_CONTEXT}: {detail}"
    ))
}
