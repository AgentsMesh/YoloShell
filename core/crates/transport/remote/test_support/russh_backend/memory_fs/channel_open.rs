use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Notify, Semaphore};

pub(super) struct ChannelOpenControl {
    pending: AtomicBool,
    accepted: AtomicBool,
    connection_closed: AtomicBool,
    connection_outcome: Mutex<Option<Result<(), russh::Error>>>,
    pending_notice: Notify,
    accepted_notice: Notify,
    closed_notice: Notify,
    release: Semaphore,
}

impl Default for ChannelOpenControl {
    fn default() -> Self {
        Self {
            pending: AtomicBool::default(),
            accepted: AtomicBool::default(),
            connection_closed: AtomicBool::default(),
            connection_outcome: Mutex::new(None),
            pending_notice: Notify::new(),
            accepted_notice: Notify::new(),
            closed_notice: Notify::new(),
            release: Semaphore::new(INITIAL_COUNTER),
        }
    }
}

impl MemoryFilesystem {
    pub async fn wait_channel_open_pending(&self) {
        let control = self.channel_open();
        wait_flag(&control.pending, &control.pending_notice).await;
    }

    pub async fn wait_channel_open_accepted(&self) {
        let control = self.channel_open();
        wait_flag(&control.accepted, &control.accepted_notice).await;
    }

    pub async fn wait_connection_closed(&self) {
        let control = self.channel_open();
        wait_flag(&control.connection_closed, &control.closed_notice).await;
    }

    pub fn release_channel_open(&self) {
        self.channel_open().release.add_permits(COUNTER_INCREMENT);
    }

    pub(crate) async fn permit_channel_open(&self) -> bool {
        let control = self.channel_open();
        control.pending.store(true, Ordering::Release);
        control.pending_notice.notify_one();
        tokio::select! {
            biased;
            _ = wait_flag(&control.connection_closed, &control.closed_notice) => false,
            permit = control.release.acquire() => {
                permit.unwrap().forget();
                true
            }
        }
    }

    pub(crate) fn record_channel_open_accepted(&self) {
        let control = self.channel_open();
        control.accepted.store(true, Ordering::Release);
        control.accepted_notice.notify_one();
    }

    pub fn connection_outcome(&self) -> String {
        format!(
            "{:?}",
            self.channel_open().connection_outcome.lock().unwrap()
        )
    }

    pub(crate) fn record_connection_closed(&self, outcome: Result<(), russh::Error>) {
        let control = self.channel_open();
        eprintln!("memory SSH server terminated: {outcome:?}");
        *control.connection_outcome.lock().unwrap() = Some(outcome);
        control.connection_closed.store(true, Ordering::Release);
        control.closed_notice.notify_one();
    }

    fn channel_open(&self) -> Arc<ChannelOpenControl> {
        Arc::clone(&self.0.lock().unwrap().channel_open)
    }
}

async fn wait_flag(flag: &AtomicBool, notice: &Notify) {
    loop {
        let changed = notice.notified();
        if flag.load(Ordering::Acquire) {
            return;
        }
        changed.await;
    }
}
