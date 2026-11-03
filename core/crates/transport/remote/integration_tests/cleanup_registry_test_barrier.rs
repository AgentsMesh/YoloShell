use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Semaphore;

const TEST_BARRIER_RELEASE_PERMITS: usize = 1;

#[derive(Default)]
struct RegistryTestState {
    next_task_barrier: Mutex<Option<Arc<CleanupRegistryBarrier>>>,
    next_wait_barrier: Mutex<Option<Arc<CleanupRegistryBarrier>>>,
}

pub(crate) struct CleanupRegistryBarrier {
    entered: AtomicBool,
    entered_notice: Notify,
    release: Semaphore,
}

impl CleanupRegistryBarrier {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entered: AtomicBool::new(false),
            entered_notice: Notify::new(),
            release: Semaphore::new(usize::default()),
        })
    }

    pub(crate) async fn wait_entered(&self) {
        loop {
            let entered = self.entered_notice.notified();
            if self.entered.load(Ordering::Acquire) {
                return;
            }
            entered.await;
        }
    }

    pub(crate) fn release(&self) {
        self.release.add_permits(TEST_BARRIER_RELEASE_PERMITS);
    }
}

impl RemoteCleanupRegistry {
    pub(crate) fn block_next_task_for_test(&self) -> Arc<CleanupRegistryBarrier> {
        let barrier = CleanupRegistryBarrier::new();
        let replaced = self
            .inner
            .test
            .next_task_barrier
            .lock()
            .unwrap()
            .replace(Arc::clone(&barrier));
        assert!(replaced.is_none());
        barrier
    }

    pub(crate) fn block_next_wait_for_test(&self) -> Arc<CleanupRegistryBarrier> {
        let barrier = CleanupRegistryBarrier::new();
        let replaced = self
            .inner
            .test
            .next_wait_barrier
            .lock()
            .unwrap()
            .replace(Arc::clone(&barrier));
        assert!(replaced.is_none());
        barrier
    }

    async fn pause_before_changed_await_for_test(&self) {
        let barrier = self.inner.test.next_wait_barrier.lock().unwrap().take();
        wait_at_barrier(barrier).await;
    }
}

async fn cleanup_after_barrier<F>(
    future: F,
    barrier: Option<Arc<CleanupRegistryBarrier>>,
) -> CleanupResult
where
    F: Future<Output = CleanupResult>,
{
    wait_at_barrier(barrier).await;
    future.await
}

async fn wait_at_barrier(barrier: Option<Arc<CleanupRegistryBarrier>>) {
    if let Some(barrier) = barrier {
        barrier.entered.store(true, Ordering::Release);
        barrier.entered_notice.notify_one();
        barrier.release.acquire().await.unwrap().forget();
    }
}
