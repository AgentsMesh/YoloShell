use super::super::{BrokerClient, BrokerConfig, BrokerPaths, Result};
use super::fixture;
use super::support::{RuntimeFixture, running_fixture, serve_ready};
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::{FakePending, TrackedBrokerTask, TrackedTaskSettlement};
use crate::start::{BrokerSpawner, reconcile};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use yolo_broker_supervision_state::ChildLaunch;

const TASK_CANCELLED_DETAIL: &str = "tracked broker task was unexpectedly cancelled";
const TASK_JOIN_FAILURE: &str = "tracked broker task join failed";

struct TrackedSpawner {
    fail_once: AtomicBool,
    calls: AtomicUsize,
    revisions: Mutex<Vec<String>>,
    tasks: Mutex<Vec<TrackedBrokerTask>>,
    source_update: Mutex<Option<(std::path::PathBuf, serde_json::Value)>>,
}

impl TrackedSpawner {
    fn fail_once() -> Self {
        Self {
            fail_once: AtomicBool::new(true),
            calls: AtomicUsize::new(0),
            revisions: Mutex::new(Vec::new()),
            tasks: Mutex::new(Vec::new()),
            source_update: Mutex::new(None),
        }
    }

    fn with_source_update(path: std::path::PathBuf, value: serde_json::Value) -> Self {
        let mut spawner = Self::fail_once();
        spawner.fail_once.store(false, Ordering::Relaxed);
        *spawner.source_update.get_mut().unwrap() = Some((path, value));
        spawner
    }

    async fn finish(&self) {
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        for task in tasks {
            match task.settle().await {
                Ok(TrackedTaskSettlement::Completed(result)) => result.unwrap(),
                Ok(TrackedTaskSettlement::Cancelled) => panic!("{TASK_CANCELLED_DETAIL}"),
                Err(error) => panic!("{TASK_JOIN_FAILURE}: {error}"),
            }
        }
    }
}

impl BrokerSpawner for TrackedSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail_once.swap(false, Ordering::Relaxed) {
            anyhow::bail!("configured broker spawn failure");
        }
        let config = BrokerConfig::load(&options.config())?;
        self.revisions
            .lock()
            .unwrap()
            .push(config.revision().to_owned());
        if let Some((path, value)) = self.source_update.lock().unwrap().take() {
            fs::write(path, serde_json::to_vec(&value)?)?;
        }
        let paths = BrokerPaths::new(&options.state_dir())?;
        anyhow::ensure!(
            yolo_remote_broker::broker_instance_available(&paths)?,
            "spawn occurred before the previous broker released its lock"
        );
        let (pending, task) = FakePending::tracked_task(serve_ready(options.clone()));
        self.tasks.lock().unwrap().push(task);
        Ok(pending)
    }
}

impl Drop for TrackedSpawner {
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

fn config_for_port(fixture: &RuntimeFixture, port: u16) -> serde_json::Value {
    let bytes = fs::read(&fixture.options.config_source).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value[fixture::ENDPOINT_FIELD][fixture::PORT_FIELD] = serde_json::json!(port);
    value
}

async fn shutdown(socket: std::path::PathBuf, revision: String) {
    BrokerClient::new(socket).shutdown(revision).await.unwrap();
}

#[tokio::test]
async fn failed_spawn_retries_only_after_the_old_broker_releases_its_lock() {
    let (fixture, old_task) = running_fixture().await;
    let value = config_for_port(&fixture, fixture::REPLACEMENT_PORT);
    fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let expected = BrokerConfig::load(&fixture.options.config_source)
        .unwrap()
        .revision()
        .to_owned();
    let spawner = TrackedSpawner::fail_once();
    reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(spawner.calls.load(Ordering::Relaxed), 2);
    assert_eq!(spawner.revisions.lock().unwrap().as_slice(), [expected]);
    old_task.await.unwrap().unwrap();
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    shutdown(
        paths.management_socket(),
        BrokerConfig::load(&fixture.options.config_source)
            .unwrap()
            .revision()
            .into(),
    )
    .await;
    spawner.finish().await;
}

#[tokio::test]
async fn replacement_keeps_its_snapshot_when_the_source_changes_mid_handoff() {
    let (fixture, old_task) = running_fixture().await;
    let desired_a = config_for_port(&fixture, fixture::REPLACEMENT_PORT);
    fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&desired_a).unwrap(),
    )
    .unwrap();
    let revision_a = BrokerConfig::load(&fixture.options.config_source)
        .unwrap()
        .revision()
        .to_owned();
    let desired_b = config_for_port(&fixture, fixture::REPLACEMENT_PORT + 1);
    let spawner = Arc::new(TrackedSpawner::with_source_update(
        fixture.options.config_source.clone(),
        desired_b,
    ));
    reconcile(
        fixture.options.clone(),
        spawner.as_ref(),
        fixture::START_TIMEOUT,
    )
    .await
    .unwrap();
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    assert_eq!(
        BrokerClient::new(paths.management_socket())
            .status()
            .await
            .unwrap()
            .config_revision,
        revision_a
    );
    assert_eq!(spawner.calls.load(Ordering::Relaxed), 1);
    old_task.await.unwrap().unwrap();
    shutdown(paths.management_socket(), revision_a).await;
    spawner.finish().await;
}
