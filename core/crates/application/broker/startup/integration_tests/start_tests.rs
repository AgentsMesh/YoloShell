use super::super::start::{BrokerSpawner, reconcile};
use super::super::{BrokerClient, BrokerConfig, BrokerPaths, Result, StartRequest};
use super::fixture;
use super::support::{RuntimeFixture, running_fixture, serve_ready};
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::{FakePending, TrackedBrokerTask, TrackedTaskSettlement};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use yolo_broker_supervision_state::ChildLaunch;

const TASK_CANCELLED_DETAIL: &str = "in-process broker task was cancelled";
const TASK_JOIN_FAILURE: &str = "in-process broker task join failed";
const MINIMUM_SPAWN_ATTEMPTS: usize = 1;
const REPLACEMENT_FAILURE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Default)]
struct InProcessSpawner {
    calls: AtomicUsize,
    tasks: Mutex<Vec<TrackedBrokerTask>>,
}

impl BrokerSpawner for InProcessSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let options = options.clone();
        let (pending, task) = FakePending::tracked_task(serve_ready(options));
        self.tasks.lock().unwrap().push(task);
        Ok(pending)
    }
}

impl Drop for InProcessSpawner {
    fn drop(&mut self) {
        let tasks = self
            .tasks
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for task in tasks.iter() {
            task.abort();
        }
    }
}

impl InProcessSpawner {
    async fn finish(&self) -> Vec<Result<()>> {
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.settle().await {
                Ok(TrackedTaskSettlement::Completed(result)) => results.push(result),
                Ok(TrackedTaskSettlement::Cancelled) => {
                    results.push(Err(anyhow::anyhow!(TASK_CANCELLED_DETAIL)));
                }
                Err(error) => panic!("{TASK_JOIN_FAILURE}: {error}"),
            }
        }
        results
    }
}

#[derive(Default)]
struct NoopSpawner(AtomicUsize);

impl BrokerSpawner for NoopSpawner {
    fn spawn(&self, _: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(FakePending::unmanaged())
    }
}

fn change_endpoint(fixture: &RuntimeFixture) {
    let bytes = fs::read(&fixture.options.config_source).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value[fixture::ENDPOINT_FIELD][fixture::PORT_FIELD] =
        serde_json::json!(fixture::REPLACEMENT_PORT);
    fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}

async fn shutdown(options: &StartRequest) {
    let config = BrokerConfig::load(&options.config_source).unwrap();
    let socket = BrokerPaths::new(&options.broker_state)
        .unwrap()
        .management_socket();
    BrokerClient::new(socket)
        .shutdown(config.revision().into())
        .await
        .unwrap();
}

#[tokio::test]
async fn same_revision_is_idempotent_without_respawn() {
    let (fixture, task) = running_fixture().await;
    let spawner = NoopSpawner::default();
    let ready = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(spawner.0.load(Ordering::Relaxed), usize::default());
    let status = BrokerClient::new(
        BrokerPaths::new(&fixture.options.broker_state)
            .unwrap()
            .management_socket(),
    )
    .status()
    .await
    .unwrap();
    assert_eq!(ready, status);
    shutdown(&fixture.options).await;
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn changed_revision_gracefully_replaces_broker() {
    let (fixture, old_task) = running_fixture().await;
    change_endpoint(&fixture);
    let spawner = InProcessSpawner::default();
    reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(spawner.calls.load(Ordering::Relaxed), 1);
    old_task.await.unwrap().unwrap();
    shutdown(&fixture.options).await;
    assert_eq!(
        spawner
            .finish()
            .await
            .iter()
            .filter(|item| item.is_ok())
            .count(),
        1
    );
}

#[tokio::test]
async fn concurrent_starts_have_one_serving_winner() {
    let fixture = RuntimeFixture::new();
    let spawner = Arc::new(InProcessSpawner::default());
    let first = reconcile(
        fixture.options.clone(),
        spawner.as_ref(),
        fixture::START_TIMEOUT,
    );
    let second = reconcile(
        fixture.options.clone(),
        spawner.as_ref(),
        fixture::START_TIMEOUT,
    );
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();
    shutdown(&fixture.options).await;
    let winners = spawner
        .finish()
        .await
        .into_iter()
        .filter(Result::is_ok)
        .count();
    assert_eq!(winners, 1);
}

#[tokio::test]
async fn replacement_failure_never_reports_target_ready() {
    let (fixture, old_task) = running_fixture().await;
    let old_revision = BrokerConfig::load(&fixture.options.config_source)
        .unwrap()
        .revision()
        .to_owned();
    change_endpoint(&fixture);
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    BrokerClient::new(paths.management_socket())
        .shutdown(old_revision)
        .await
        .unwrap();
    old_task.await.unwrap().unwrap();

    let spawner = NoopSpawner::default();
    let error = reconcile(
        fixture.options.clone(),
        &spawner,
        REPLACEMENT_FAILURE_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert!(
        spawner.0.load(Ordering::Relaxed) >= MINIMUM_SPAWN_ATTEMPTS,
        "reconciliation failed before spawn: {error:#}"
    );
}
