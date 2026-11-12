use super::super::connection_bootstrap::CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE;
use super::super::timing::CONNECTION_BOOTSTRAP_TIMEOUT;
use super::super::*;
use super::support::*;
use crate::remote::{RemoteCleanupRegistry, RemoteError};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIRST_CONNECTION_INDEX: usize = 0;
const SECOND_CONNECTION_INDEX: usize = FIRST_CONNECTION_INDEX + 1;
const EXPECTED_BOOTSTRAP_CONNECTIONS: usize = 1;
const DEPLOYMENT_FAILURE_DETAIL: &str = "runtime deployment interrupted";

fn pool<F>(factory: Arc<F>) -> BrokerConnectionPool
where
    F: BrokerConnectionFactory + 'static,
{
    let config = test_config();
    BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    )
}

#[tokio::test(start_paused = true)]
async fn bootstrap_operation_timeout_invalidates_without_reconnecting() {
    let stalled = FakeConnection::new([OpenStep::StalledChannel]);
    let replacement = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&stalled), replacement],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(SECOND_CONNECTION_INDEX),
        ],
    );
    let shared = Arc::new(pool(Arc::clone(&factory)));
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.bootstrap().await });
    while stalled.opens().is_empty() {
        tokio::task::yield_now().await;
    }

    tokio::time::advance(crate::remote::SSH_OPERATION_TIMEOUT).await;
    let error = task.await.unwrap().unwrap_err();

    assert!(matches!(
        error,
        RemoteError::Transport(message)
            if message == crate::remote::OPERATION_TIMEOUT_MESSAGE
    ));
    assert_eq!(factory.calls(), EXPECTED_BOOTSTRAP_CONNECTIONS);
    assert!(shared.status().await.connection_id.is_none());
}

struct DeploymentFailureFactory(AtomicUsize);

impl BrokerConnectionFactory for DeploymentFailureFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.0
                .fetch_add(EXPECTED_BOOTSTRAP_CONNECTIONS, Ordering::Relaxed);
            Err(RemoteError::RuntimeDeployment(
                DEPLOYMENT_FAILURE_DETAIL.into(),
            ))
        })
    }
}

#[tokio::test]
async fn deployment_connection_loss_is_terminal_without_same_call_reupload() {
    let factory = Arc::new(DeploymentFailureFactory(AtomicUsize::default()));
    let shared = pool(factory.clone());

    let error = shared.bootstrap().await.unwrap_err();

    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(detail) if detail == DEPLOYMENT_FAILURE_DETAIL
    ));
    assert_eq!(
        factory.0.load(Ordering::Relaxed),
        EXPECTED_BOOTSTRAP_CONNECTIONS
    );
}

#[tokio::test(start_paused = true)]
async fn bootstrap_transport_retries_share_one_absolute_ceiling() {
    let factory = FakeFactory::empty(Vec::new());
    let shared = Arc::new(pool(Arc::clone(&factory)));
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.bootstrap().await });
    while factory.calls() == usize::default() {
        tokio::task::yield_now().await;
    }

    tokio::time::advance(CONNECTION_BOOTSTRAP_TIMEOUT).await;
    let error = task.await.unwrap().unwrap_err();

    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message)
            if message == CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE
    ));
    assert!(factory.calls() > EXPECTED_BOOTSTRAP_CONNECTIONS);
    assert!(shared.status().await.connection_id.is_none());
}
