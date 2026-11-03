use super::*;
use crate::coordination::CoordinationLock;
use crate::paths::directory::PrivateDirectory;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const AVAILABLE_OWNER_FILE: &str = "available-owner.lock";
const BLOCKED_OWNER_FILE: &str = "blocked-owner.lock";
const DEFAULT_GROUP_COUNT: usize = 2;
const GLOBAL_GROUP_INDEX: usize = 1;
const NO_SCANNER_CALLS: usize = 0;
const OWNER_RECORD_MAX_BYTES: u64 = 0;
const OVERRIDE_GROUP_COUNT: usize = 1;
const SOCKET_GROUP_INDEX: usize = 0;
const SOCKET_GROUP_SCANNER_COUNT: usize = 1;
const SOCKET_ROOT_DIRECTORY: &str = "socket-root";
const TEST_WAIT: Duration = Duration::from_secs(3);
const TEST_SCANNER_LABEL: &str = "isolation-test";

struct CountingScanner {
    calls: Arc<AtomicUsize>,
}

impl MaintenanceScanner for CountingScanner {
    fn label(&self) -> &'static str {
        TEST_SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        Ok(ScanChunk::PassComplete)
    }

    fn reset(&mut self) {}
}

#[test]
fn socket_isolation_override_plan_excludes_uid_global_scanners() {
    let directory = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&directory.path().join(SOCKET_ROOT_DIRECTORY)).unwrap();
    let scanner = crate::paths::test_support::scanner_for_override_root(&root).unwrap();

    let plan = maintenance_plan_with_socket_scanner(scanner).unwrap();

    assert_eq!(plan.groups.len(), OVERRIDE_GROUP_COUNT);
    assert_eq!(
        plan.groups[SOCKET_GROUP_INDEX].scanners.len(),
        SOCKET_GROUP_SCANNER_COUNT
    );
}

#[test]
fn socket_isolation_default_plan_keeps_global_owner_separate() {
    let directory = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&directory.path().join(SOCKET_ROOT_DIRECTORY)).unwrap();
    let scanner = crate::paths::test_support::scanner_for_default_root(&root).unwrap();

    let plan = maintenance_plan_with_socket_scanner(scanner).unwrap();

    assert_eq!(plan.groups.len(), DEFAULT_GROUP_COUNT);
    assert_ne!(
        plan.groups[SOCKET_GROUP_INDEX].owner_path,
        plan.groups[GLOBAL_GROUP_INDEX].owner_path
    );
}

#[tokio::test]
async fn socket_isolation_contended_group_does_not_block_another_owner() {
    let _worker_test = test_support::serialize_worker_tests().await;
    let directory = tempfile::tempdir().unwrap();
    let blocked_owner = directory.path().join(BLOCKED_OWNER_FILE);
    let available_owner = directory.path().join(AVAILABLE_OWNER_FILE);
    let _contention = CoordinationLock::try_acquire(&blocked_owner, OWNER_RECORD_MAX_BYTES)
        .unwrap()
        .unwrap();
    let blocked_calls = Arc::new(AtomicUsize::new(0));
    let available_calls = Arc::new(AtomicUsize::new(0));
    let plan = MaintenancePlan {
        groups: vec![
            group(&blocked_owner, Arc::clone(&blocked_calls)),
            group(&available_owner, Arc::clone(&available_calls)),
        ],
    };

    let mut worker = MaintenanceWorker::start(plan);
    let deadline = Instant::now() + TEST_WAIT;
    while available_calls.load(Ordering::Acquire) == NO_SCANNER_CALLS && Instant::now() < deadline {
        std::thread::yield_now();
    }
    worker.shutdown().await.unwrap();

    assert_eq!(blocked_calls.load(Ordering::Acquire), NO_SCANNER_CALLS);
    assert!(available_calls.load(Ordering::Acquire) > NO_SCANNER_CALLS);
}

fn group(path: &std::path::Path, calls: Arc<AtomicUsize>) -> MaintenanceGroup {
    MaintenanceGroup {
        owner_path: path.to_path_buf(),
        scanners: vec![Box::new(CountingScanner { calls })],
    }
}
