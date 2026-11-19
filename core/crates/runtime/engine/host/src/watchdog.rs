use super::*;

use std::future::Future;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub(super) const WATCHDOG_FACTORY_PANIC_MESSAGE: &str = "host recovery watchdog factory panicked";
pub(super) const WATCHDOG_SWEEP_COMPLETE_MESSAGE: &str = "host recovery watchdog sweep completed";
pub(super) const WATCHDOG_SWEEP_PANIC_MESSAGE: &str = "host recovery watchdog sweep panicked";

#[derive(Clone, Copy, Debug)]
pub(super) struct WatchdogSchedule {
    pub(super) interval: Duration,
    pub(super) failure_backoff: Duration,
}

impl Default for WatchdogSchedule {
    fn default() -> Self {
        Self {
            interval: WATCHDOG_INTERVAL,
            failure_backoff: WATCHDOG_FAILURE_BACKOFF,
        }
    }
}

pub(super) struct WatchdogGuard {
    pub(super) shutdown: Option<oneshot::Sender<()>>,
    pub(super) task: JoinHandle<()>,
}

impl WatchdogGuard {
    pub(super) async fn cancel_and_wait(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let _ = (&mut self.task).await;
    }
}

impl Drop for WatchdogGuard {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

impl Host {
    pub(super) fn spawn_watchdog(self: &Arc<Self>) -> WatchdogGuard {
        let (shutdown, receiver) = oneshot::channel();
        let host = Arc::clone(self);
        let schedule = self.watchdog_schedule;
        let task = tokio::spawn(run_watchdog(schedule, receiver, move || {
            let host = Arc::clone(&host);
            async move { host.reconcile().await }
        }));
        WatchdogGuard {
            shutdown: Some(shutdown),
            task,
        }
    }
}

pub(super) async fn run_watchdog<F, Fut, T>(
    schedule: WatchdogSchedule,
    mut shutdown: oneshot::Receiver<()>,
    sweep: F,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = T>,
    T: std::fmt::Debug,
{
    let mut sweep = terminal_containment::DropGuard::new(sweep);
    let mut delay = Duration::ZERO;
    let mut attempt = u64::default();
    loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => return,
            _ = tokio::time::sleep(delay) => {}
        }
        attempt = attempt.saturating_add(1);
        let future = match terminal_containment::catch(|| sweep.get_mut()()) {
            Ok(future) => future,
            Err(()) => {
                terminal_containment::trace(|| {
                    tracing::error!(attempt, WATCHDOG_FACTORY_PANIC_MESSAGE);
                });
                delay = schedule.failure_backoff;
                continue;
            }
        };
        let outcome = tokio::select! {
            biased;
            _ = &mut shutdown => return,
            result = terminal_containment::catch_future(future) => result,
        };
        delay = match outcome {
            Ok(result) => {
                let detail = terminal_containment::debug_detail(result);
                terminal_containment::trace(|| {
                    tracing::trace!(attempt, %detail, WATCHDOG_SWEEP_COMPLETE_MESSAGE);
                });
                schedule.interval
            }
            Err(()) => {
                terminal_containment::trace(|| {
                    tracing::error!(attempt, WATCHDOG_SWEEP_PANIC_MESSAGE);
                });
                schedule.failure_backoff
            }
        };
    }
}

pub(super) fn contain_panic<F: Future>(future: F) -> impl Future<Output = Result<F::Output, ()>> {
    terminal_containment::catch_future(future)
}
