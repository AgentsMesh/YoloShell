use anyhow::Result;
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

const REAPER_INIT_FAILURE: &str = "initialize broker termination reaper";
const REAPER_QUEUE_FULL: &str = "broker termination custody queue is full";
const REAPER_WORKER_FAILED: &str = "broker termination reaper worker failed";
const TERMINATION_QUEUE_CAPACITY: usize = 32;
const TERMINATION_THREAD_NAME: &str = "yolo-broker-termination-reaper";

#[path = "spawned_termination_containment.rs"]
mod containment;
#[path = "spawned_termination_custody.rs"]
mod custody;
#[path = "spawned_termination_worker.rs"]
pub(super) mod worker;

#[derive(Clone)]
struct TerminationReaper {
    shared: Arc<Shared>,
}

pub(super) struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
}

#[derive(Default)]
pub(super) struct Queue {
    children: VecDeque<RetainedChild<yolo_process::DetachedChildCustody>>,
    custody_count: usize,
    termination_count: usize,
    worker_failed: bool,
}

pub(super) struct RetainedChild<C = yolo_process::DetachedChildCustody> {
    pub(super) child: C,
    pub(super) termination_required: bool,
}

#[derive(Clone)]
struct InitializationFailure {
    kind: io::ErrorKind,
    detail: String,
}

pub struct TerminationReservation {
    shared: Arc<Shared>,
    active: bool,
}

static REAPER: OnceLock<std::result::Result<TerminationReaper, InitializationFailure>> =
    OnceLock::new();

pub fn initialize() -> Result<()> {
    current().map(|_| ())
}

pub fn reserve() -> Result<TerminationReservation> {
    current()?.reserve()
}

pub use custody::{wait_for_custody_settlement, wait_for_termination_delivery};

fn current() -> Result<&'static TerminationReaper> {
    match REAPER.get_or_init(|| TerminationReaper::start().map_err(InitializationFailure::from)) {
        Ok(reaper) => Ok(reaper),
        Err(error) => Err(io::Error::new(error.kind, error.detail.clone()))
            .map_err(anyhow::Error::new)
            .map_err(|error| error.context(REAPER_INIT_FAILURE)),
    }
}

impl TerminationReaper {
    fn start() -> io::Result<Self> {
        Self::start_with(|shared| {
            std::thread::Builder::new()
                .name(TERMINATION_THREAD_NAME.to_owned())
                .spawn(move || worker::run(shared))
                .map(|_| ())
        })
    }

    fn start_with<L>(launch: L) -> io::Result<Self>
    where
        L: FnOnce(Arc<Shared>) -> io::Result<()>,
    {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
        });
        launch(Arc::clone(&shared))?;
        Ok(Self { shared })
    }

    fn reserve(&self) -> Result<TerminationReservation> {
        let mut queue = lock_queue(&self.shared);
        anyhow::ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
        anyhow::ensure!(
            queue.custody_count < TERMINATION_QUEUE_CAPACITY,
            REAPER_QUEUE_FULL
        );
        queue.custody_count += 1;
        drop(queue);
        Ok(TerminationReservation {
            shared: Arc::clone(&self.shared),
            active: true,
        })
    }
}

impl From<io::Error> for InitializationFailure {
    fn from(error: io::Error) -> Self {
        Self {
            kind: error.kind(),
            detail: error.to_string(),
        }
    }
}

pub(super) fn lock_queue(shared: &Shared) -> std::sync::MutexGuard<'_, Queue> {
    shared
        .queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/spawned_termination_containment_tests.rs"]
mod containment_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/spawned_termination_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/reaper_custody.rs"]
mod custody_tests;
