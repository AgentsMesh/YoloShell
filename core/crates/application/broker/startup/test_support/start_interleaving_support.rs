use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::{FakePending, TrackedBrokerTask, TrackedTaskSettlement};
use crate::shared_test_support::serve_ready;
use crate::start::BrokerSpawner;
use crate::{BrokerConfig, BrokerPaths, Result};
use std::fs;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use yolo_broker_supervision_state::ChildLaunch;

const DUPLICATE_ADMISSION_DETAIL: &str = "duplicate broker launch admission";
const LAUNCH_INCREMENT: usize = 1;
const LOCK_HANDOFF_DETAIL: &str = "spawn occurred before broker lock handoff";
const TASK_COUNT_OVERFLOW_DETAIL: &str = "successful broker task counter overflow";
const TASK_JOIN_FAILURE: &str = "in-process broker task join failed";
const TASK_SERVE_FAILURE: &str = "in-process broker task failed";

struct Gate {
    entered: Sender<()>,
    release: Receiver<()>,
}

impl Gate {
    fn new(entered: Sender<()>, release: Receiver<()>) -> Self {
        Self { entered, release }
    }

    fn wait(self) -> Result<()> {
        self.entered.send(())?;
        self.release.recv()?;
        Ok(())
    }
}

pub(super) struct Spawner {
    calls: AtomicUsize,
    revisions: Mutex<Vec<String>>,
    tasks: Mutex<Vec<TrackedBrokerTask>>,
    source_update: Mutex<Option<(std::path::PathBuf, serde_json::Value)>>,
    call_gate: Mutex<Option<Gate>>,
}

impl Spawner {
    pub(super) fn call_blocked(
        source: std::path::PathBuf,
        update: serde_json::Value,
        entered: Sender<()>,
        release: Receiver<()>,
    ) -> Self {
        Self::new(Some((source, update)), Some(Gate::new(entered, release)))
    }

    pub(super) fn ready() -> Self {
        Self::new(None, None)
    }

    fn new(
        source_update: Option<(std::path::PathBuf, serde_json::Value)>,
        call_gate: Option<Gate>,
    ) -> Self {
        Self {
            calls: AtomicUsize::new(usize::default()),
            revisions: Mutex::new(Vec::new()),
            tasks: Mutex::new(Vec::new()),
            source_update: Mutex::new(source_update),
            call_gate: Mutex::new(call_gate),
        }
    }

    pub(super) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    pub(super) fn revisions(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.revisions.lock().unwrap()
    }

    pub(super) async fn successful_task_count(&self) -> usize {
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        let mut successes = usize::default();
        for task in tasks {
            match task.settle().await.expect(TASK_JOIN_FAILURE) {
                TrackedTaskSettlement::Completed(Ok(())) => {
                    successes = successes
                        .checked_add(LAUNCH_INCREMENT)
                        .expect(TASK_COUNT_OVERFLOW_DETAIL);
                }
                TrackedTaskSettlement::Completed(Err(error)) => {
                    panic!("{TASK_SERVE_FAILURE}: {error:#}")
                }
                TrackedTaskSettlement::Cancelled => {}
            }
        }
        successes
    }
}

impl BrokerSpawner for Spawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        let previous = self.calls.fetch_add(LAUNCH_INCREMENT, Ordering::SeqCst);
        anyhow::ensure!(previous == usize::default(), DUPLICATE_ADMISSION_DETAIL);
        self.revisions
            .lock()
            .unwrap()
            .push(BrokerConfig::load(&options.config())?.revision().into());
        if let Some((path, value)) = self.source_update.lock().unwrap().take() {
            fs::write(path, serde_json::to_vec(&value)?)?;
        }
        if let Some(gate) = self.call_gate.lock().unwrap().take() {
            gate.wait()?;
        }
        let paths = BrokerPaths::new(&options.state_dir())?;
        anyhow::ensure!(
            yolo_remote_broker::broker_instance_available(&paths)?,
            LOCK_HANDOFF_DETAIL
        );
        let (pending, task) = FakePending::tracked_task(serve_ready(options.clone()));
        self.tasks.lock().unwrap().push(task);
        Ok(pending)
    }
}

impl Drop for Spawner {
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
