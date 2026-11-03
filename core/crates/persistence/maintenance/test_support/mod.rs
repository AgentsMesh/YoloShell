use super::{MaintenanceScanner, ScanChunk};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

const FAILSAFE_REAPER_ERROR: &str = "await scanner failsafe reaper";
const FAILSAFE_THREAD_ERROR: &str = "scanner failsafe thread panicked";
static WORKER_TEST_GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn serialize_worker_tests() -> tokio::sync::MutexGuard<'static, ()> {
    WORKER_TEST_GATE.lock().await
}

struct Gate {
    released: Mutex<bool>,
    wake: Condvar,
    forced: AtomicBool,
}

pub struct ScannerBarrier {
    gate: Arc<Gate>,
    entered: Option<oneshot::Receiver<()>>,
    dropped: Option<oneshot::Receiver<()>>,
}

pub struct BlockingScanner {
    label: &'static str,
    gate: Arc<Gate>,
    entered: Option<oneshot::Sender<()>>,
    dropped: Option<oneshot::Sender<()>>,
}

impl ScannerBarrier {
    pub fn new(label: &'static str) -> (Self, BlockingScanner) {
        let gate = Arc::new(Gate {
            released: Mutex::new(false),
            wake: Condvar::new(),
            forced: AtomicBool::new(false),
        });
        let (entered_tx, entered_rx) = oneshot::channel();
        let (dropped_tx, dropped_rx) = oneshot::channel();
        (
            Self {
                gate: Arc::clone(&gate),
                entered: Some(entered_rx),
                dropped: Some(dropped_rx),
            },
            BlockingScanner {
                label,
                gate,
                entered: Some(entered_tx),
                dropped: Some(dropped_tx),
            },
        )
    }

    pub async fn wait_entered(&mut self) {
        self.entered
            .take()
            .expect("scanner entry receiver is single-use")
            .await
            .expect("scanner entry sender was dropped");
    }

    pub async fn wait_dropped(&mut self) {
        self.dropped
            .take()
            .expect("scanner drop receiver is single-use")
            .await
            .expect("scanner drop sender was dropped");
    }

    pub fn release(&self) {
        let mut released = self
            .gate
            .released
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *released = true;
        self.gate.wake.notify_all();
    }

    pub fn arm_failsafe(&self, timeout: Duration) -> std::thread::JoinHandle<()> {
        let gate = Arc::clone(&self.gate);
        std::thread::spawn(move || {
            let released = gate
                .released
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let (mut released, _) = gate
                .wake
                .wait_timeout_while(released, timeout, |released| !*released)
                .unwrap_or_else(|error| error.into_inner());
            if !*released {
                gate.forced.store(true, Ordering::Release);
                *released = true;
                gate.wake.notify_all();
            }
        })
    }

    pub fn was_forced(&self) -> bool {
        self.gate.forced.load(Ordering::Acquire)
    }
}

pub async fn settle_failsafe(failsafe: std::thread::JoinHandle<()>) -> Result<()> {
    let panicked = tokio::task::spawn_blocking(move || failsafe.join().is_err())
        .await
        .map_err(|error| anyhow::anyhow!("{FAILSAFE_REAPER_ERROR}: {error}"))?;
    anyhow::ensure!(!panicked, FAILSAFE_THREAD_ERROR);
    Ok(())
}

impl MaintenanceScanner for BlockingScanner {
    fn label(&self) -> &'static str {
        self.label
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        if let Some(entered) = self.entered.take() {
            let _ = entered.send(());
        }
        let released = self
            .gate
            .released
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        drop(
            self.gate
                .wake
                .wait_while(released, |released| !*released)
                .unwrap_or_else(|error| error.into_inner()),
        );
        Ok(ScanChunk::PassComplete)
    }

    fn reset(&mut self) {}
}

impl Drop for BlockingScanner {
    fn drop(&mut self) {
        if let Some(dropped) = self.dropped.take() {
            let _ = dropped.send(());
        }
    }
}
