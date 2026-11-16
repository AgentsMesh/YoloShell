use super::super::connection_bootstrap::CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE;
use super::super::timing::CONNECTION_BOOTSTRAP_TIMEOUT;
use super::super::*;
use super::support::*;
use crate::remote::{MultiplexedSshConnection, RemoteCleanupRegistry, RemoteError, RuntimeRole};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

const CLOSED_GATE_PERMITS: usize = 0;
const CONNECT_CALL_INCREMENT: usize = 1;
const EXPECTED_CONNECT_CALLS: usize = 1;
const INITIAL_CONNECTION_GENERATION: u64 = 0;
const NO_CONNECT_CALLS: usize = 0;
const SINGLE_GATE_PERMIT: usize = 1;

struct PublicationFactory {
    connection: Arc<dyn MultiplexedSshConnection>,
    entered: Semaphore,
    release: Semaphore,
    returned: Semaphore,
    calls: AtomicUsize,
}

impl PublicationFactory {
    fn new(connection: Arc<FakeConnection>) -> Arc<Self> {
        Arc::new(Self {
            connection,
            entered: Semaphore::new(CLOSED_GATE_PERMITS),
            release: Semaphore::new(CLOSED_GATE_PERMITS),
            returned: Semaphore::new(CLOSED_GATE_PERMITS),
            calls: AtomicUsize::new(NO_CONNECT_CALLS),
        })
    }
}

impl BrokerConnectionFactory for PublicationFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.calls
                .fetch_add(CONNECT_CALL_INCREMENT, Ordering::Relaxed);
            self.entered.add_permits(SINGLE_GATE_PERMIT);
            self.release.acquire().await.unwrap().forget();
            self.returned.add_permits(SINGLE_GATE_PERMIT);
            Ok(Arc::clone(&self.connection))
        })
    }
}

fn publication_pool(factory: Arc<PublicationFactory>) -> Arc<BrokerConnectionPool> {
    let config = test_config();
    Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ))
}

#[tokio::test(start_paused = true)]
async fn bootstrap_deadline_cancels_factory_before_first_state_publication() {
    let connection = FakeConnection::new([OpenStep::Channel]);
    let factory = PublicationFactory::new(connection.clone());
    let shared = publication_pool(factory.clone());
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    factory.entered.acquire().await.unwrap().forget();

    let state_entered = Arc::new(Semaphore::new(CLOSED_GATE_PERMITS));
    let state_release = Arc::new(Semaphore::new(CLOSED_GATE_PERMITS));
    let blocker_pool = Arc::clone(&shared);
    let blocker_entered = Arc::clone(&state_entered);
    let blocker_release = Arc::clone(&state_release);
    let blocker = tokio::spawn(async move {
        blocker_pool
            .block_state_updates(blocker_entered, blocker_release)
            .await;
    });
    state_entered.acquire().await.unwrap().forget();
    factory.release.add_permits(SINGLE_GATE_PERMIT);
    factory.returned.acquire().await.unwrap().forget();
    tokio::time::advance(CONNECTION_BOOTSTRAP_TIMEOUT).await;

    let error = match task.await.unwrap() {
        Ok(_) => panic!("timed-out publication must not return a channel"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message)
            if message == CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE
    ));
    state_release.add_permits(SINGLE_GATE_PERMIT);
    blocker.await.unwrap();
    tokio::task::yield_now().await;
    let status = shared.status().await;
    assert!(status.connection_id.is_none());
    assert_eq!(status.connection_generation, INITIAL_CONNECTION_GENERATION);
    assert!(connection.opens().is_empty());
    assert_eq!(
        factory.calls.load(Ordering::Relaxed),
        EXPECTED_CONNECT_CALLS
    );
}
