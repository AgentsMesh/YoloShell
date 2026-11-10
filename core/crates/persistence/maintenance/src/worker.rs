use super::control::StopControl;
use super::scanner_loop::run_as_owner;
use super::{MaintenanceGroup, MaintenancePlan, WORKER_SHUTDOWN_GRACE_PERIOD};
use crate::coordination::CoordinationLock;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const OWNER_RECORD_MAX_BYTES: u64 = 0;
#[cfg(not(test))]
const OWNER_RETRY_INTERVAL: Duration = Duration::from_secs(5);
#[cfg(test)]
const OWNER_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const ERROR_RETRY_INITIAL: Duration = Duration::from_millis(25);
const ERROR_RETRY_MAX: Duration = Duration::from_secs(5);
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(10);
const THREAD_NAME: &str = "yolo-maintenance";
const JOIN_CONTEXT: &str = "join Yolo maintenance worker";
const SHUTDOWN_TIMEOUT_CONTEXT: &str = "Yolo maintenance worker shutdown timed out";
pub(super) const RESET_PANIC_CONTEXT: &str = "maintenance scanner reset panicked";

pub struct MaintenanceWorker {
    control: Arc<StopControl>,
    reset_panicked: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

struct Schedule {
    next: Instant,
    error_delay: Duration,
}

impl Schedule {
    fn ready() -> Self {
        Self {
            next: Instant::now(),
            error_delay: ERROR_RETRY_INITIAL,
        }
    }

    fn after_success(&mut self, delay: Duration) {
        self.next = Instant::now() + delay;
        self.error_delay = ERROR_RETRY_INITIAL;
    }

    fn after_failure(&mut self) {
        self.next = Instant::now() + self.error_delay;
        self.error_delay = self
            .error_delay
            .saturating_add(self.error_delay)
            .min(ERROR_RETRY_MAX);
    }
}

impl MaintenanceWorker {
    pub fn start(plan: MaintenancePlan) -> Self {
        let control = Arc::new(StopControl::new());
        let reset_panicked = Arc::new(AtomicBool::new(false));
        let threads = plan
            .groups
            .into_iter()
            .filter_map(|group| {
                let worker_control = Arc::clone(&control);
                let worker_reset_panicked = Arc::clone(&reset_panicked);
                std::thread::Builder::new()
                    .name(THREAD_NAME.into())
                    .spawn(move || run(group, &worker_control, &worker_reset_panicked))
                    .map_err(|error| tracing::warn!(%error, "maintenance worker unavailable"))
                    .ok()
            })
            .collect();
        Self {
            control,
            reset_panicked,
            threads,
        }
    }

    pub fn request_shutdown(&self) {
        self.control.request();
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_with_grace(WORKER_SHUTDOWN_GRACE_PERIOD).await
    }

    #[cfg(test)]
    pub(crate) async fn shutdown_with_grace_for_test(
        &mut self,
        shutdown_grace_period: Duration,
    ) -> Result<()> {
        self.shutdown_with_grace(shutdown_grace_period).await
    }

    async fn shutdown_with_grace(&mut self, shutdown_grace_period: Duration) -> Result<()> {
        self.request_shutdown();
        let deadline = Instant::now() + shutdown_grace_period;
        let mut threads = std::mem::take(&mut self.threads);
        let mut panicked = false;
        loop {
            let mut pending = Vec::with_capacity(threads.len());
            for thread in threads {
                if thread.is_finished() {
                    panicked |= thread.join().is_err();
                } else {
                    pending.push(thread);
                }
            }
            if pending.is_empty() {
                break;
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    workers = pending.len(),
                    "maintenance workers exceeded shutdown grace period"
                );
                drop(pending);
                anyhow::bail!(SHUTDOWN_TIMEOUT_CONTEXT);
            }
            threads = pending;
            tokio::time::sleep(SHUTDOWN_POLL_INTERVAL).await;
        }
        anyhow::ensure!(!panicked, JOIN_CONTEXT);
        anyhow::ensure!(
            !self.reset_panicked.load(Ordering::Acquire),
            RESET_PANIC_CONTEXT
        );
        Ok(())
    }
}

impl Drop for MaintenanceWorker {
    fn drop(&mut self) {
        self.request_shutdown();
        // Dropping JoinHandle detaches the stop-requested thread. A canceled async
        // shutdown moved its handles into the blocking reaper before suspending.
        self.threads.clear();
    }
}

fn run(mut group: MaintenanceGroup, control: &StopControl, reset_panicked: &AtomicBool) {
    let mut owner_errors = Schedule::ready();
    while !control.requested() {
        match CoordinationLock::try_acquire(&group.owner_path, OWNER_RECORD_MAX_BYTES) {
            Ok(Some(_owner)) => {
                run_as_owner(&mut group.scanners, control, reset_panicked);
                if group.scanners.is_empty() {
                    return;
                }
            }
            Ok(None) => {
                owner_errors.after_success(OWNER_RETRY_INTERVAL);
                control.wait(OWNER_RETRY_INTERVAL);
            }
            Err(error) => {
                tracing::warn!(%error, "maintenance owner election failed");
                owner_errors.after_failure();
                control.wait(owner_errors.next.saturating_duration_since(Instant::now()));
            }
        }
    }
}
