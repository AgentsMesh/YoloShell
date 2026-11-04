use super::constants::*;
use std::sync::{Arc, Mutex};
use yolo_remote::{
    IdentityFile, KnownHosts, MultiplexedSshConnection, RemoteCleanupRegistry, RemoteEndpoint,
    RusshConnection, SshConnectConfig,
};
use yolo_remote_broker::{BrokerConnectFuture, BrokerConnectionFactory};

pub(crate) struct LiveSshFactory {
    endpoint: RemoteEndpoint,
    known_hosts: KnownHosts,
    identity: IdentityFile,
    first: Mutex<Option<Arc<dyn MultiplexedSshConnection>>>,
}

impl LiveSshFactory {
    pub(crate) fn new(
        endpoint: RemoteEndpoint,
        known_hosts: KnownHosts,
        identity: IdentityFile,
    ) -> Arc<Self> {
        Arc::new(Self {
            endpoint,
            known_hosts,
            identity,
            first: Mutex::new(None),
        })
    }

    pub(crate) fn seed(&self, connection: Arc<dyn MultiplexedSshConnection>) {
        let previous = self.first.lock().unwrap().replace(connection);
        assert!(previous.is_none(), "SSH factory was seeded twice");
    }

    pub(crate) async fn connect_once(
        &self,
        cleanup: RemoteCleanupRegistry,
    ) -> Result<Arc<dyn MultiplexedSshConnection>, yolo_remote::RemoteError> {
        let config = SshConnectConfig::new(self.endpoint.clone(), self.known_hosts.clone())
            .use_agent(false)
            .with_identity(self.identity.clone())
            .with_timeouts(SSH_CONNECT_TIMEOUT, SSH_INACTIVITY_TIMEOUT)
            .with_operation_timeout(SSH_OPERATION_TIMEOUT)
            .with_cleanup_registry(cleanup);
        Ok(Arc::new(RusshConnection::connect(config).await?) as Arc<dyn MultiplexedSshConnection>)
    }
}

impl BrokerConnectionFactory for LiveSshFactory {
    fn connect_with_cleanup(&self, cleanup: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        let first = self.first.lock().unwrap().take();
        Box::pin(async move {
            match first {
                Some(connection) => Ok(connection),
                None => self.connect_once(cleanup).await,
            }
        })
    }
}
