use super::*;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const ERROR_TRIGGER: &str = "error-before-reset";
const DROP_PANIC: &str = "injected scanner drop panic";
const NO_CALLS: usize = 0;
const ONE_CALL: usize = 1;
const OWNER_FILE: &str = "reset-panic-owner.lock";
const PANIC_TRIGGER: &str = "panic-before-reset";
const RESET_PANIC: &str = "injected reset panic";
const SIBLING_LABEL: &str = "reset-panic-sibling";
const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const TWO_SIBLING_PASSES: usize = 2;

#[derive(Clone, Copy)]
enum ScanFailure {
    Error,
    Panic,
}

struct ResetPanicScanner {
    calls: Arc<AtomicUsize>,
    failure: ScanFailure,
    resets: Arc<AtomicUsize>,
}

struct SiblingScanner {
    calls: Arc<AtomicUsize>,
}

impl MaintenanceScanner for ResetPanicScanner {
    fn label(&self) -> &'static str {
        match self.failure {
            ScanFailure::Error => ERROR_TRIGGER,
            ScanFailure::Panic => PANIC_TRIGGER,
        }
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        match self.failure {
            ScanFailure::Error => anyhow::bail!(ERROR_TRIGGER),
            ScanFailure::Panic => panic!("{PANIC_TRIGGER}"),
        }
    }

    fn reset(&mut self) {
        self.resets.fetch_add(1, Ordering::AcqRel);
        panic!("{RESET_PANIC}");
    }
}

impl Drop for ResetPanicScanner {
    fn drop(&mut self) {
        if matches!(self.failure, ScanFailure::Panic) {
            panic!("{DROP_PANIC}");
        }
    }
}

impl MaintenanceScanner for SiblingScanner {
    fn label(&self) -> &'static str {
        SIBLING_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        Ok(ScanChunk::PassComplete)
    }

    fn reset(&mut self) {}
}

#[tokio::test]
async fn scan_error_then_reset_panic_quarantines_only_the_failed_scanner() {
    assert_reset_panic_isolated(ScanFailure::Error).await;
}

#[tokio::test]
async fn scan_panic_then_reset_panic_quarantines_only_the_failed_scanner() {
    assert_reset_panic_isolated(ScanFailure::Panic).await;
}

async fn assert_reset_panic_isolated(failure: ScanFailure) {
    let _worker_test = test_support::serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let failed_calls = Arc::new(AtomicUsize::new(NO_CALLS));
    let resets = Arc::new(AtomicUsize::new(NO_CALLS));
    let sibling_calls = Arc::new(AtomicUsize::new(NO_CALLS));
    let plan = MaintenancePlan::single(
        root.path().join(OWNER_FILE),
        vec![
            Box::new(ResetPanicScanner {
                calls: Arc::clone(&failed_calls),
                failure,
                resets: Arc::clone(&resets),
            }),
            Box::new(SiblingScanner {
                calls: Arc::clone(&sibling_calls),
            }),
        ],
    );
    let mut worker = MaintenanceWorker::start(plan);
    let deadline = Instant::now() + TEST_TIMEOUT;
    while sibling_calls.load(Ordering::Acquire) < TWO_SIBLING_PASSES && Instant::now() < deadline {
        std::thread::yield_now();
    }

    let error = worker.shutdown().await.unwrap_err();

    assert!(error.to_string().contains(worker::RESET_PANIC_CONTEXT));
    assert_eq!(failed_calls.load(Ordering::Acquire), ONE_CALL);
    assert_eq!(resets.load(Ordering::Acquire), ONE_CALL);
    assert!(sibling_calls.load(Ordering::Acquire) >= TWO_SIBLING_PASSES);
}
