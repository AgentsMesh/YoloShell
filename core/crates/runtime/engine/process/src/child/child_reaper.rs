use anyhow::{Context, Result, ensure};
use std::collections::VecDeque;
use std::io;
use std::process::Child;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

const REAPER_INIT_CONTEXT: &str = "initialize detached child reaper";
const REAPER_QUEUE_FULL: &str = "detached child custody queue is full";
const REAPER_WORKER_FAILED: &str = "detached child reaper worker failed";
const REAPER_THREAD_NAME: &str = "yolo-detached-child-reaper";
const REAPER_QUEUE_CAPACITY: usize = 64;
pub(super) const REAPER_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[path = "child_reaper_containment.rs"]
mod containment;
#[path = "child_reaper_custody.rs"]
pub(super) mod custody;
#[path = "child_reaper_error.rs"]
mod error;
#[path = "child_reaper_reserved.rs"]
mod reserved;
#[path = "child_reaper_worker.rs"]
pub(super) mod worker;
pub use error::DetachedChildHandoffError;
use error::InitializationFailure;
pub use reserved::{ReservedDetachedChildSettlement, reserve_detached_child_settlement};

static REAPER: OnceLock<std::result::Result<DetachedChildReaper, InitializationFailure>> =
    OnceLock::new();

#[derive(Clone)]
struct DetachedChildReaper {
    shared: Arc<Shared>,
}

pub(super) struct Shared {
    pub(super) queue: Mutex<Queue>,
    pub(super) ready: Condvar,
    #[cfg(test)]
    pub(super) panic_after_receive: AtomicBool,
    #[cfg(test)]
    pub(super) panic_payload_drops: Arc<AtomicUsize>,
}

#[derive(Default)]
pub(super) struct Queue {
    pub(super) children: VecDeque<RetainedChild<super::DetachedChildCustody>>,
    pub(super) custody_count: usize,
    pub(super) termination_count: usize,
    pub(super) worker_failed: bool,
}

pub(super) struct RetainedChild<C> {
    pub(super) child: C,
    pub(super) policy: TerminationPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminationPolicy {
    NaturalExit,
    Terminate,
}

pub(super) struct DetachedChildReservation {
    shared: Arc<Shared>,
    active: bool,
}

pub(super) fn prepare_detached_child_handoff() -> Result<DetachedChildReservation> {
    current()?.reserve()
}

pub fn handoff_detached_child(child: Child) -> Result<(), DetachedChildHandoffError> {
    let custody = match super::DetachedChildCustody::capture(child) {
        Ok(custody) => custody,
        Err(error) => {
            let (source, child) = error.into_parts();
            return Err(DetachedChildHandoffError::new(source, child));
        }
    };
    handoff_detached_child_custody(custody)
}

pub fn handoff_detached_child_custody(
    custody: super::DetachedChildCustody,
) -> Result<(), DetachedChildHandoffError> {
    handoff_custody_with_reservation(custody, prepare_detached_child_handoff())
}

#[cfg(test)]
fn handoff_with_reservation(
    child: Child,
    reservation: Result<DetachedChildReservation>,
) -> Result<(), DetachedChildHandoffError> {
    let custody = match super::DetachedChildCustody::capture(child) {
        Ok(custody) => custody,
        Err(error) => {
            let (source, child) = error.into_parts();
            return Err(DetachedChildHandoffError::new(source, child));
        }
    };
    handoff_custody_with_reservation(custody, reservation)
}

fn handoff_custody_with_reservation(
    custody: super::DetachedChildCustody,
    reservation: Result<DetachedChildReservation>,
) -> Result<(), DetachedChildHandoffError> {
    let reservation = match reservation {
        Ok(reservation) => reservation,
        Err(source) => {
            return Err(DetachedChildHandoffError::new(source, custody.release()));
        }
    };
    reservation.retain(custody, TerminationPolicy::NaturalExit);
    Ok(())
}

fn current() -> Result<&'static DetachedChildReaper> {
    match REAPER.get_or_init(|| DetachedChildReaper::start().map_err(InitializationFailure::from)) {
        Ok(reaper) => Ok(reaper),
        Err(error) => Err(io::Error::new(error.kind, error.detail.clone()))
            .map_err(anyhow::Error::new)
            .context(REAPER_INIT_CONTEXT),
    }
}

impl DetachedChildReaper {
    fn start() -> io::Result<Self> {
        Self::start_with(|shared| {
            std::thread::Builder::new()
                .name(REAPER_THREAD_NAME.to_owned())
                .spawn(move || worker::run(shared))
                .map(|_| ())
        })
    }

    fn start_with(launch: impl FnOnce(Arc<Shared>) -> io::Result<()>) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
            #[cfg(test)]
            panic_after_receive: AtomicBool::new(false),
            #[cfg(test)]
            panic_payload_drops: Arc::new(AtomicUsize::new(usize::default())),
        });
        launch(Arc::clone(&shared))?;
        Ok(Self { shared })
    }

    fn reserve(&self) -> Result<DetachedChildReservation> {
        let mut queue = lock_queue(&self.shared);
        ensure!(!queue.worker_failed, REAPER_WORKER_FAILED);
        ensure!(
            queue.custody_count < REAPER_QUEUE_CAPACITY,
            REAPER_QUEUE_FULL
        );
        queue.custody_count += 1;
        drop(queue);
        Ok(DetachedChildReservation {
            shared: Arc::clone(&self.shared),
            active: true,
        })
    }
}

pub(super) fn lock_queue(shared: &Shared) -> std::sync::MutexGuard<'_, Queue> {
    shared
        .queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn is_child_already_reaped(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ECHILD)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/child/child_reaper_panic_tests.rs"]
mod panic_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/child/child_reaper_reserved_tests.rs"]
mod reserved_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/child/child_reaper_state_tests.rs"]
mod state_tests;
