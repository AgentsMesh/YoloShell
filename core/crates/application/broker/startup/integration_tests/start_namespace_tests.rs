use super::super::Result;
use super::fixture;
use super::support::RuntimeFixture;
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::FakePending;
use crate::start::{BrokerSpawner, reconcile};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::BrokerPaths;

const DISPLACED_STATE_DIRECTORY: &str = "spawn-displaced-state";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";
const COMPENSATION_FAILURE: &str = "injected compensation failure";
const NAMESPACE_FAILURE: &str = "namespace identity was lost";

struct ReplacementSpawner {
    compensated: Arc<AtomicBool>,
    sentinel: Mutex<Option<PathBuf>>,
    fail_compensation: bool,
}

impl ReplacementSpawner {
    fn new(fail_compensation: bool) -> Self {
        Self {
            compensated: Arc::new(AtomicBool::new(false)),
            sentinel: Mutex::new(None),
            fail_compensation,
        }
    }
}

impl BrokerSpawner for ReplacementSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        let displaced = options
            .state_dir()
            .parent()
            .unwrap()
            .join(DISPLACED_STATE_DIRECTORY);
        fs::rename(&options.state_dir(), displaced)?;
        let replacement = BrokerPaths::new(&options.state_dir())?;
        let sentinel = replacement.state_dir().join(SENTINEL_FILE);
        fs::write(&sentinel, SENTINEL_BYTES)?;
        self.sentinel.lock().unwrap().replace(sentinel);
        let compensated = Arc::clone(&self.compensated);
        let fail = self.fail_compensation;
        Ok(FakePending::tracked(move || {
            compensated.store(true, Ordering::SeqCst);
            anyhow::ensure!(!fail, COMPENSATION_FAILURE);
            Ok(())
        }))
    }
}

async fn replacement_result(
    fail_compensation: bool,
) -> (RuntimeFixture, ReplacementSpawner, anyhow::Error) {
    let fixture = RuntimeFixture::new();
    let spawner = ReplacementSpawner::new(fail_compensation);
    let error = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap_err();
    (fixture, spawner, error)
}

#[tokio::test]
async fn namespace_loss_after_spawn_invokes_compensation() {
    let (_fixture, spawner, error) = replacement_result(false).await;

    assert!(error.to_string().contains(NAMESPACE_FAILURE), "{error:#}");
    assert!(spawner.compensated.load(Ordering::SeqCst));
    assert_eq!(
        fs::read(spawner.sentinel.lock().unwrap().as_ref().unwrap()).unwrap(),
        SENTINEL_BYTES
    );
}

#[tokio::test]
async fn compensation_failure_does_not_hide_namespace_loss() {
    let (_fixture, spawner, error) = replacement_result(true).await;

    let message = error.to_string();
    assert!(message.contains(NAMESPACE_FAILURE), "{message}");
    assert!(message.contains(COMPENSATION_FAILURE), "{message}");
    assert!(spawner.compensated.load(Ordering::SeqCst));
}
