use super::test_support::{TestRoot, scanner};
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const MISSING_EXECUTABLE: &str = "/yolo-test-missing/process-snapshot-helper";
const ROOT_GENERATION_OFFSET: u64 = 1;
const HEALTHY_SCAN_BUDGET: Duration = Duration::from_secs(5);
const ADVERSARIAL_SCAN_BUDGET: Duration = Duration::from_millis(250);
const CHILD_OBSERVATION_BUDGET: Duration = Duration::from_millis(500);
const CHILD_NOT_SPAWNED: &str = "isolated scanner helper was not spawned";

fn deadline(budget: Duration) -> Instant {
    Instant::now() + budget
}

#[tokio::test]
async fn isolated_scanner_returns_stable_receipt_from_real_helper() {
    let root = TestRoot::spawn();
    let observation = scanner()
        .scan(root.identity, deadline(HEALTHY_SCAN_BUDGET))
        .await
        .unwrap();

    assert!(matches!(
        observation.descendants,
        FencedDescendants::Stable(_)
    ));
    assert!(observation.started_at_ms > 0);
    assert!(observation.completed_at_ms >= observation.started_at_ms);
}

#[tokio::test]
async fn isolated_scanner_reports_root_generation_change() {
    let root = TestRoot::spawn();
    let mut changed = root.identity;
    changed.start_time = changed.start_time.saturating_add(ROOT_GENERATION_OFFSET);

    let observation = scanner()
        .scan(changed, deadline(HEALTHY_SCAN_BUDGET))
        .await
        .unwrap();

    assert!(matches!(
        observation.descendants,
        FencedDescendants::RootChanged
    ));
}

#[tokio::test]
async fn isolated_scanner_rejects_hangs_before_and_after_a_frame() {
    use super::super::process_snapshot_fault as behavior;
    let root = TestRoot::spawn();
    for behavior in [behavior::HANG_BEFORE_FRAME, behavior::HANG_AFTER_FRAME] {
        let result = scanner()
            .with_behavior(behavior)
            .scan(root.identity, deadline(ADVERSARIAL_SCAN_BUDGET))
            .await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn isolated_scanner_rejects_nonzero_exit_after_a_valid_frame() {
    let root = TestRoot::spawn();
    let result = scanner()
        .with_behavior(super::super::process_snapshot_fault::FAIL_AFTER_FRAME)
        .scan(root.identity, deadline(HEALTHY_SCAN_BUDGET))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn isolated_scanner_rejects_expired_deadline_and_spawn_failure() {
    let root = TestRoot::spawn();
    assert!(scanner().scan(root.identity, Instant::now()).await.is_err());
    let missing = IsolatedProcessScanner::new(MISSING_EXECUTABLE.into(), Vec::new()).unwrap();
    assert!(
        missing
            .scan(root.identity, deadline(HEALTHY_SCAN_BUDGET))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn cancelling_hung_scan_does_not_block_a_following_scan() {
    let root = TestRoot::spawn();
    let spawned_process = Arc::new(AtomicU32::new(u32::default()));
    let hung = Arc::new(
        scanner()
            .with_behavior(super::super::process_snapshot_fault::HANG_BEFORE_FRAME)
            .with_spawn_observer(spawned_process.clone()),
    );
    let task_scanner = Arc::clone(&hung);
    let task = tokio::spawn(async move {
        task_scanner
            .scan(root.identity, deadline(HEALTHY_SCAN_BUDGET))
            .await
    });
    let helper = observe_spawned_process(&spawned_process).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    super::test_support::await_child_reaped(helper).await;

    assert!(
        scanner()
            .scan(root.identity, deadline(HEALTHY_SCAN_BUDGET))
            .await
            .is_ok()
    );
}

async fn observe_spawned_process(observer: &AtomicU32) -> u32 {
    tokio::time::timeout(CHILD_OBSERVATION_BUDGET, async {
        loop {
            let pid = observer.load(Ordering::Acquire);
            if pid != u32::default() {
                return pid;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect(CHILD_NOT_SPAWNED)
}

#[cfg(target_os = "linux")]
#[path = "isolated_process_scanner_foreground_linux_tests.rs"]
mod foreground_linux;
