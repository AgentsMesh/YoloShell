use super::super::{BrokerPaths, Result};
use super::fixture;
use super::support::RuntimeFixture;
use crate::pending::PendingSpawnHandle;
use crate::start::{BrokerSpawner, reconcile};
use std::sync::Mutex;
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::{BROKER_READY_POLL_INTERVAL, BrokerLaunchDeadline, BrokerReconcileGuard};

const NEAR_LOCK_POLL_WINDOWS: u32 = 2;
const EXPECTED_LAUNCH_ATTEMPTS: usize = 1;
const FORCED_SPAWN_FAILURE: &str = "forced deadline spawn failure";
const RECONCILIATION_TIMEOUT_DETAIL: &str = "broker did not reach";

#[derive(Default)]
struct DeadlineSpawner(Mutex<Vec<tokio::time::Instant>>);

impl BrokerSpawner for DeadlineSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        let deadline = options.startup_deadline().to_instant()?;
        self.0.lock().unwrap().push(deadline);
        anyhow::bail!(FORCED_SPAWN_FAILURE)
    }
}

#[tokio::test(start_paused = true)]
async fn near_exhausted_lock_cannot_refresh_the_child_or_reconcile_budget() {
    // Bind the process-wide anchor to real time without changing the paused timer origin.
    std::thread::spawn(|| {
        assert!(tokio::runtime::Handle::try_current().is_err());
        BrokerLaunchDeadline::capture(tokio::time::Instant::now() + fixture::START_TIMEOUT)
    })
    .join()
    .unwrap()
    .unwrap();
    let fixture = RuntimeFixture::new();
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    let held =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + fixture::START_TIMEOUT)
            .await
            .unwrap();
    let spawner = std::sync::Arc::new(DeadlineSpawner::default());
    let task_spawner = std::sync::Arc::clone(&spawner);
    let options = fixture.options.clone();
    let started = tokio::time::Instant::now();
    let task = tokio::spawn(async move {
        reconcile(options, task_spawner.as_ref(), fixture::START_TIMEOUT).await
    });
    tokio::task::yield_now().await;
    let remaining = BROKER_READY_POLL_INTERVAL * NEAR_LOCK_POLL_WINDOWS;
    tokio::time::advance(fixture::START_TIMEOUT.saturating_sub(remaining)).await;
    drop(held);
    tokio::time::advance(BROKER_READY_POLL_INTERVAL).await;

    let error = task.await.unwrap().unwrap_err();
    let launches = spawner.0.lock().unwrap();

    assert!(error.to_string().contains(RECONCILIATION_TIMEOUT_DETAIL));
    assert_eq!(launches.len(), EXPECTED_LAUNCH_ATTEMPTS);
    assert!(launches[0] <= started + fixture::START_TIMEOUT);
    assert!(launches[0].duration_since(tokio::time::Instant::now()) <= remaining);
    assert_eq!(
        tokio::time::Instant::now(),
        started + fixture::START_TIMEOUT
    );
}
