use super::super::connection_bootstrap::CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE;
use super::super::connection_runtime::RUNTIME_OPEN_TIMEOUT_MESSAGE;
use super::super::constants::MAX_RECONNECT_DELAY;
use super::super::timing::{CONNECTION_BOOTSTRAP_TIMEOUT, RUNTIME_OPEN_TIMEOUT};
use super::super::*;
use super::support::*;
use crate::remote::{
    MultiplexedSshConnection, RemoteCleanupRegistry, RemoteError, RuntimeRole,
    SSH_OPERATION_TIMEOUT,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;

const CLOSED_GATE_PERMITS: usize = 0;
const CONNECT_CALL_INCREMENT: usize = 1;
const EXPECTED_FIRST_CALL: usize = 1;
const EXPECTED_EXPLICIT_RETRY_CALLS: usize = 2;
const LOSS_STEPS_BEYOND_RUNTIME_DEADLINE: usize = 64;
const NO_CONNECT_CALLS: usize = 0;
const SINGLE_GATE_PERMIT: usize = 1;
const RUNTIME_FAILURE_REQUIRED: &str = "runtime open unexpectedly succeeded";

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
            entered: Semaphore::new(CLOSED_GATE_PERMITS),
            release: Semaphore::new(CLOSED_GATE_PERMITS),
            calls: AtomicUsize::new(NO_CONNECT_CALLS),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl BrokerConnectionFactory for BlockingFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.calls
                .fetch_add(CONNECT_CALL_INCREMENT, Ordering::Relaxed);
            self.entered.add_permits(SINGLE_GATE_PERMIT);
            self.release.acquire().await.unwrap().forget();
            Ok(Arc::clone(&self.connection))
        })
    }
}

struct RepeatingFactory {
    connection: Arc<dyn MultiplexedSshConnection>,
    calls: AtomicUsize,
}

impl BrokerConnectionFactory for RepeatingFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.calls
                .fetch_add(CONNECT_CALL_INCREMENT, Ordering::Relaxed);
            Ok(Arc::clone(&self.connection))
        })
    }
}

fn pool(factory: Arc<dyn BrokerConnectionFactory>) -> Arc<BrokerConnectionPool> {
    let config = test_config();
    Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ))
}

fn assert_bootstrap_timeout(error: &RemoteError) {
    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message)
            if message == CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE
    ));
}

type RuntimeOpenTask = tokio::task::JoinHandle<Result<yolo_runtime_rpc::RuntimeConnection, RemoteError>>;
async fn expire_bootstrap(task: RuntimeOpenTask) {
    tokio::task::yield_now().await;
    tokio::time::advance(CONNECTION_BOOTSTRAP_TIMEOUT).await;
    assert_bootstrap_timeout(&task.await.unwrap().err().unwrap());
}

#[tokio::test(start_paused = true)]
async fn progressing_bootstrap_outlives_channel_open_budget() {
    let _clock = super::runtime_client::DrivenClock::new();
    let factory = BlockingFactory::new(FakeConnection::new([]));
    let shared = pool(factory.clone());
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    factory.entered.acquire().await.unwrap().forget();

    tokio::time::advance(SSH_OPERATION_TIMEOUT).await;
    assert!(!task.is_finished());
    factory.release.add_permits(SINGLE_GATE_PERMIT);

    drop(task.await.unwrap().unwrap());
    assert_eq!(factory.calls(), EXPECTED_FIRST_CALL);
    assert!(shared.status().await.connection_id.is_some());
}

#[tokio::test(start_paused = true)]
async fn bootstrap_timeout_cancels_once_and_requires_an_explicit_retry() {
    let _clock = super::runtime_client::DrivenClock::new();
    let factory = BlockingFactory::new(FakeConnection::new([]));
    let shared = pool(factory.clone());
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    factory.entered.acquire().await.unwrap().forget();

    expire_bootstrap(task).await;
    assert_eq!(factory.calls(), EXPECTED_FIRST_CALL);
    assert!(shared.status().await.connection_id.is_none());

    factory.release.add_permits(SINGLE_GATE_PERMIT);
    drop(shared.open_runtime(RuntimeRole::Control).await.unwrap());
    assert_eq!(factory.calls(), EXPECTED_EXPLICIT_RETRY_CALLS);
}

#[tokio::test(start_paused = true)]
async fn bootstrap_deadline_bounds_transport_retries() {
    let _clock = super::runtime_client::DrivenClock::new();
    let factory = FakeFactory::empty(Vec::new());
    let task_pool = pool(factory.clone());
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    while factory.calls() == NO_CONNECT_CALLS {
        tokio::task::yield_now().await;
    }

    expire_bootstrap(task).await;
    assert!(factory.calls() > EXPECTED_FIRST_CALL);
}

#[tokio::test(start_paused = true)]
async fn bootstrap_deadline_includes_waiting_for_the_reconnect_gate() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([]);
    let factory = BlockingFactory::new(connection.clone());
    let shared = pool(factory.clone());
    let holder_pool = Arc::clone(&shared);
    let holder = tokio::spawn(async move { holder_pool.connection().await });
    factory.entered.acquire().await.unwrap().forget();
    let waiter = tokio::spawn(async move { shared.open_runtime(RuntimeRole::Control).await });

    expire_bootstrap(waiter).await;
    assert_eq!(factory.calls(), EXPECTED_FIRST_CALL);
    factory.release.add_permits(SINGLE_GATE_PERMIT);
    assert_eq!(
        holder.await.unwrap().unwrap().info().connection_id,
        connection.id()
    );
}

#[path = "runtime_bootstrap_retry_tests.rs"]
mod retries;
