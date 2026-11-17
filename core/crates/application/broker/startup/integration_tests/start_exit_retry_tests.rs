use super::super::{BrokerClient, BrokerConfig, BrokerPaths, Result};
use super::fixture;
use super::support::{RuntimeFixture, running_fixture, serve_ready};
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::{FakePending, TrackedBrokerTask, TrackedTaskSettlement};
use crate::start::{BrokerSpawner, reconcile};
use anyhow::Context;
use std::fs;
use std::process::Command;
use std::sync::Mutex;
use yolo_broker_supervision_state::ChildLaunch;

const ATTEMPT_INCREMENT: usize = 1;
const SERVING_ATTEMPT_INDEX: usize = 1;
const EXPECTED_LAUNCH_ATTEMPTS: usize = 2;
const EXPECTED_SERVING_TASKS: usize = 1;
const ATTEMPT_OVERFLOW_DETAIL: &str = "exit retry launch counter overflow";
const DUPLICATE_SERVE_DETAIL: &str = "exit retry admitted more than one serving launch";
const TASK_CANCELLED_DETAIL: &str = "exit retry broker task was cancelled";
const TASK_JOIN_FAILURE: &str = "exit retry broker task join failed";

#[derive(Default)]
struct ExitThenServeSpawner {
    calls: Mutex<usize>,
    tasks: Mutex<Vec<TrackedBrokerTask>>,
}

impl ExitThenServeSpawner {
    fn next_attempt(&self) -> Result<usize> {
        let mut calls = self.calls.lock().unwrap();
        let attempt = *calls;
        *calls = calls
            .checked_add(ATTEMPT_INCREMENT)
            .context(ATTEMPT_OVERFLOW_DETAIL)?;
        Ok(attempt)
    }

    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }

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

impl BrokerSpawner for ExitThenServeSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        let attempt = self.next_attempt()?;
        if attempt == usize::default() {
            let child = Command::new(fixture::TEST_FAILURE_EXECUTABLE).spawn()?;
            return Ok(FakePending::detached_for_test(child));
        }
        anyhow::ensure!(attempt == SERVING_ATTEMPT_INDEX, DUPLICATE_SERVE_DETAIL);
        let (pending, task) = FakePending::tracked_task(serve_ready(options.clone()));
        self.tasks.lock().unwrap().push(task);
        Ok(pending)
    }
}

impl Drop for ExitThenServeSpawner {
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

fn replacement_config(fixture: &RuntimeFixture) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.options.config_source).unwrap()).unwrap();
    value[fixture::ENDPOINT_FIELD][fixture::PORT_FIELD] =
        serde_json::json!(fixture::REPLACEMENT_PORT);
    value
}

#[tokio::test]
async fn retry_requires_observing_the_exact_spawned_child_exit() {
    let (fixture, old_task) = running_fixture().await;
    fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&replacement_config(&fixture)).unwrap(),
    )
    .unwrap();
    let revision = BrokerConfig::load(&fixture.options.config_source)
        .unwrap()
        .revision()
        .to_owned();
    let spawner = ExitThenServeSpawner::default();

    reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();

    assert_eq!(spawner.calls(), EXPECTED_LAUNCH_ATTEMPTS);
    old_task.await.unwrap().unwrap();
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    BrokerClient::new(paths.management_socket())
        .shutdown(revision)
        .await
        .unwrap();
    let results = spawner.finish().await;
    assert_eq!(results.len(), EXPECTED_SERVING_TASKS);
    assert!(results.into_iter().all(|result| result.is_ok()));
}
