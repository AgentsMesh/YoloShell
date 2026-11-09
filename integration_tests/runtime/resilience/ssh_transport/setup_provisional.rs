use super::super::runtime_guard::DetachedRuntimeGuard;
use super::guard::{GuardArmFailure, arm_runtime_guard};
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::oneshot;

#[cfg(test)]
#[path = "setup_probe_barrier.rs"]
mod probe_barrier;
#[cfg(test)]
pub(super) use probe_barrier::ProbeBarrier;

const PROVISIONAL_THREAD_NAME: &str = "yolo-ssh-runtime-guard";
const PROVISIONAL_THREAD_CONTEXT: &str = "spawn provisional detached runtime guard";
const PROVISIONAL_RUNTIME_CONTEXT: &str = "start provisional detached runtime guard runtime";
const PROVISIONAL_CHANNEL_CLOSED_DETAIL: &str =
    "provisional detached runtime guard stopped before publishing authority";
const PROVISIONAL_RECEIVER_MISSING_DETAIL: &str =
    "provisional detached runtime guard receiver is missing";
const UNCLAIMED_GUARD_FAILURE_DETAIL: &str =
    "unclaimed detached runtime guard could not clean its Host";
const UNCLAIMED_ARM_FAILURE_DETAIL: &str =
    "unclaimed detached runtime guard failed to bind its Host";

type GuardResult = std::result::Result<DetachedRuntimeGuard, GuardArmFailure>;

pub(super) struct ProvisionalRuntimeGuard {
    receiver: Option<oneshot::Receiver<GuardResult>>,
    cancellation: Option<oneshot::Sender<()>>,
    cancelled: Arc<AtomicBool>,
}

impl Drop for ProvisionalRuntimeGuard {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(cancellation) = self.cancellation.take() {
            let _ = cancellation.send(());
        }
    }
}

impl ProvisionalRuntimeGuard {
    pub(super) fn start(expected_home: &Path) -> Result<Self> {
        Self::start_inner(expected_home.to_path_buf(), None)
    }

    #[cfg(test)]
    pub(super) fn start_with_barrier(expected_home: &Path, barrier: ProbeBarrier) -> Result<Self> {
        Self::start_inner(expected_home.to_path_buf(), Some(barrier))
    }

    fn start_inner(expected_home: PathBuf, barrier: Option<ProbeBarrier>) -> Result<Self> {
        let (sender, receiver) = oneshot::channel();
        let (cancellation, cancellation_request) = oneshot::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker = std::thread::Builder::new()
            .name(PROVISIONAL_THREAD_NAME.into())
            .spawn(move || {
                run_worker(
                    expected_home,
                    barrier,
                    sender,
                    cancellation_request,
                    worker_cancelled,
                )
            })
            .context(PROVISIONAL_THREAD_CONTEXT)?;
        drop(worker);
        Ok(Self {
            receiver: Some(receiver),
            cancellation: Some(cancellation),
            cancelled,
        })
    }

    pub(super) async fn acquire(mut self) -> GuardResult {
        self.receiver
            .take()
            .expect(PROVISIONAL_RECEIVER_MISSING_DETAIL)
            .await
            .unwrap_or_else(|_| {
                Err(GuardArmFailure::unbound(anyhow!(
                    PROVISIONAL_CHANNEL_CLOSED_DETAIL
                )))
            })
    }
}

fn run_worker(
    expected_home: PathBuf,
    barrier: Option<ProbeBarrier>,
    sender: oneshot::Sender<GuardResult>,
    cancellation: oneshot::Receiver<()>,
    cancelled: Arc<AtomicBool>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context(PROVISIONAL_RUNTIME_CONTEXT);
    let runtime = match runtime {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = sender.send(Err(GuardArmFailure::unbound(error)));
            return;
        }
    };
    runtime.block_on(async move {
        tokio::pin!(cancellation);
        #[cfg(test)]
        let cancelled_at_barrier = if let Some(barrier) = barrier {
            tokio::select! {
                biased;
                _ = &mut cancellation => true,
                _ = barrier.pause_async(&cancelled) => false,
            }
        } else {
            false
        };
        #[cfg(not(test))]
        let cancelled_at_barrier = {
            let _ = barrier;
            false
        };
        let arming = arm_runtime_guard(&expected_home);
        if cancelled_at_barrier {
            settle_unclaimed(arming.await).await;
            return;
        }
        tokio::pin!(arming);
        tokio::select! {
            biased;
            _ = &mut cancellation => {
                settle_unclaimed(arming.await).await;
            }
            result = &mut arming => {
                if let Err(unclaimed) = sender.send(result) {
                    settle_unclaimed(unclaimed).await;
                }
            }
        }
    });
}

async fn settle_unclaimed(result: GuardResult) {
    let (binding, mut guard) = match result {
        Ok(guard) => (None, Some(guard)),
        Err(failure) => {
            let (error, guard) = failure.into_parts();
            (Some(error), guard)
        }
    };
    if guard.is_some()
        && let Some(error) = binding
    {
        tracing::error!(%error, message = UNCLAIMED_ARM_FAILURE_DETAIL);
    }
    if let Some(guard) = guard.as_mut()
        && let Err(error) = guard.shutdown().await
    {
        tracing::error!(%error, message = UNCLAIMED_GUARD_FAILURE_DETAIL);
    }
}
