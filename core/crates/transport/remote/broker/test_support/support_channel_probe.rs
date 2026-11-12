use futures::task::AtomicWaker;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Context;
use tokio::sync::{Notify, Semaphore};

pub struct ChannelProbe {
    pub writes_allowed: AtomicBool,
    write_pending: AtomicUsize,
    shutdown_pending: AtomicUsize,
    dropped: AtomicUsize,
    write_notice: Notify,
    shutdown_notice: Notify,
    drop_notice: Notify,
    write_waker: AtomicWaker,
    open_gate: Semaphore,
}

impl ChannelProbe {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            writes_allowed: AtomicBool::new(false),
            write_pending: AtomicUsize::new(0),
            shutdown_pending: AtomicUsize::new(0),
            dropped: AtomicUsize::new(0),
            write_notice: Notify::new(),
            shutdown_notice: Notify::new(),
            drop_notice: Notify::new(),
            write_waker: AtomicWaker::new(),
            open_gate: Semaphore::new(0),
        })
    }

    pub fn release_open(&self) {
        self.open_gate.add_permits(1);
    }

    pub fn release_writes(&self) {
        self.writes_allowed.store(true, Ordering::Release);
        self.write_waker.wake();
    }

    pub async fn wait_write_pending(&self, expected: usize) {
        wait_count(&self.write_pending, &self.write_notice, expected).await;
    }

    pub async fn wait_shutdown_pending(&self, expected: usize) {
        wait_count(&self.shutdown_pending, &self.shutdown_notice, expected).await;
    }

    pub async fn wait_dropped(&self, expected: usize) {
        wait_count(&self.dropped, &self.drop_notice, expected).await;
    }

    pub async fn wait_open(&self) {
        self.open_gate.acquire().await.unwrap().forget();
    }

    pub fn pending_write(&self, cx: &Context<'_>) {
        self.write_waker.register(cx.waker());
        self.write_pending.fetch_add(1, Ordering::Relaxed);
        self.write_notice.notify_waiters();
    }

    pub fn pending_shutdown(&self) {
        self.shutdown_pending.fetch_add(1, Ordering::Relaxed);
        self.shutdown_notice.notify_waiters();
    }

    pub fn dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
        self.drop_notice.notify_waiters();
    }
}

async fn wait_count(count: &AtomicUsize, notice: &Notify, expected: usize) {
    loop {
        let notified = notice.notified();
        if count.load(Ordering::Relaxed) >= expected {
            return;
        }
        notified.await;
    }
}
