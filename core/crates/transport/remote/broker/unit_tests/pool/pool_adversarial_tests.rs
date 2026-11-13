use super::super::*;
use super::support::*;
use crate::remote::{MultiplexedSshConnection, RemoteCleanupRegistry, RemoteError, RuntimeRole};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

struct BlockingFactory {
    connection: Arc<dyn MultiplexedSshConnection>,
    entered: Semaphore,
    release: Semaphore,
    calls: AtomicUsize,
}

impl BlockingFactory {
    fn new(connection: Arc<FakeConnection>) -> Arc<Self> {
        Arc::new(Self {
            connection,
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            calls: AtomicUsize::new(0),
        })
    }
}

impl BrokerConnectionFactory for BlockingFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.entered.add_permits(1);
            self.release.acquire().await.unwrap().forget();
            Ok(Arc::clone(&self.connection))
        })
    }
}

fn pool(factory: Arc<dyn BrokerConnectionFactory>) -> BrokerConnectionPool {
    let config = test_config();
    BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    )
}

#[tokio::test]
async fn waiters_reuse_connection_created_inside_single_flight_gate() {
    let connection = FakeConnection::new([]);
    let factory = BlockingFactory::new(connection.clone());
    let pool = Arc::new(pool(factory.clone()));
    let first_pool = Arc::clone(&pool);
    let first = tokio::spawn(async move { first_pool.connection().await.unwrap() });
    factory.entered.acquire().await.unwrap().forget();
    let second_pool = Arc::clone(&pool);
    let second = tokio::spawn(async move { second_pool.connection().await.unwrap() });
    tokio::task::yield_now().await;
    factory.release.add_permits(1);
    assert_eq!(first.await.unwrap().info().connection_id, connection.id());
    assert_eq!(second.await.unwrap().info().connection_id, connection.id());
    assert_eq!(factory.calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn runtime_purpose_mismatch_is_rejected_without_transport_invalidation() {
    let connection = FakeConnection::new([OpenStep::PurposeMismatch]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let pool = pool(factory);
    assert!(matches!(
        pool.open_runtime(RuntimeRole::Attachment).await,
        Err(RemoteError::ChannelMismatch)
    ));
    assert_eq!(pool.status().await.connection_id, Some(connection.id()));
}
