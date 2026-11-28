use crate::remote::{MultiplexedSshConnection, RemoteCleanupRegistry, RemoteError};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type BrokerConnectFuture<'a> = Pin<
    Box<dyn Future<Output = Result<Arc<dyn MultiplexedSshConnection>, RemoteError>> + Send + 'a>,
>;

pub trait BrokerConnectionFactory: Send + Sync {
    fn connect_with_cleanup(&self, cleanup: RemoteCleanupRegistry) -> BrokerConnectFuture<'_>;

    fn connect(&self) -> BrokerConnectFuture<'_> {
        self.connect_with_cleanup(RemoteCleanupRegistry::default())
    }
}
