use super::*;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const TEST_WAIT: Duration = Duration::from_secs(3);
const OWNER_FILE_NAME: &str = ".owner.lock";

#[test]
fn production_namespace_scanners_have_lazy_constructors() {
    drop(crate::catalog::CatalogAliasScanner::new());
    drop(crate::paths::socket_namespace::SocketNamespaceScanner::new());
}

struct TestScanner {
    calls: Arc<AtomicUsize>,
    resets: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
    panic_once: bool,
}

struct FailingScanner {
    calls: Arc<AtomicUsize>,
}

impl MaintenanceScanner for FailingScanner {
    fn label(&self) -> &'static str {
        "failing-test"
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        anyhow::bail!("injected scanner failure")
    }

    fn reset(&mut self) {}
}

struct CompleteScanner {
    calls: Arc<AtomicUsize>,
}

impl MaintenanceScanner for CompleteScanner {
    fn label(&self) -> &'static str {
        "complete-test"
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        Ok(ScanChunk::PassComplete)
    }

    fn reset(&mut self) {}
}

impl MaintenanceScanner for TestScanner {
    fn label(&self) -> &'static str {
        "test"
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if std::mem::take(&mut self.panic_once) {
            panic!("injected scanner panic");
        }
        Ok(ScanChunk::PassComplete)
    }

    fn reset(&mut self) {
        self.resets.fetch_add(1, Ordering::AcqRel);
    }
}

impl Drop for TestScanner {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

#[tokio::test]
async fn worker_recovers_scanner_panic_and_cancel_joins() {
    let _worker_test = test_support::serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let resets = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicBool::new(false));
    let scanner = TestScanner {
        calls: Arc::clone(&calls),
        resets: Arc::clone(&resets),
        dropped: Arc::clone(&dropped),
        panic_once: true,
    };
    let plan = MaintenancePlan::single(root.path().join(OWNER_FILE_NAME), vec![Box::new(scanner)]);
    let mut worker = MaintenanceWorker::start(plan);
    let deadline = Instant::now() + TEST_WAIT;
    while calls.load(Ordering::Acquire) < 2 && Instant::now() < deadline {
        std::thread::yield_now();
    }
    worker.shutdown().await.unwrap();
    assert!(calls.load(Ordering::Acquire) >= 2);
    assert_eq!(resets.load(Ordering::Acquire), 1);
    assert!(dropped.load(Ordering::Acquire));
}

#[tokio::test]
async fn shutdown_is_idempotent() {
    let _worker_test = test_support::serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let plan = MaintenancePlan::single(root.path().join(OWNER_FILE_NAME), Vec::new());
    let mut worker = MaintenanceWorker::start(plan);
    worker.shutdown().await.unwrap();
    worker.shutdown().await.unwrap();
}

#[tokio::test]
async fn failing_scanner_does_not_starve_another_scanners_passes() {
    let _worker_test = test_support::serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let failing_calls = Arc::new(AtomicUsize::new(0));
    let complete_calls = Arc::new(AtomicUsize::new(0));
    let plan = MaintenancePlan::single(
        root.path().join(OWNER_FILE_NAME),
        vec![
            Box::new(FailingScanner {
                calls: Arc::clone(&failing_calls),
            }),
            Box::new(CompleteScanner {
                calls: Arc::clone(&complete_calls),
            }),
        ],
    );
    let mut worker = MaintenanceWorker::start(plan);
    let deadline = Instant::now() + TEST_WAIT;
    while complete_calls.load(Ordering::Acquire) < 2 && Instant::now() < deadline {
        std::thread::yield_now();
    }
    worker.shutdown().await.unwrap();
    assert!(failing_calls.load(Ordering::Acquire) >= 1);
    assert!(complete_calls.load(Ordering::Acquire) >= 2);
}
