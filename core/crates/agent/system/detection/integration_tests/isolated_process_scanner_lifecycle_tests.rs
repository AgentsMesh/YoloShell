use super::test_support::{TestRoot, runtime_executable, scanner};
use super::*;
use crate::process::ScopedWaitableChild;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

const HELPER_EXIT_BUDGET: Duration = Duration::from_secs(5);
const HELPER_PROCESS_MISSING: &str = "isolated scanner helper process was not recorded";

fn deadline() -> Instant {
    Instant::now() + HELPER_EXIT_BUDGET
}

#[tokio::test]
async fn helper_reported_failure_is_retryable_and_reaped() {
    let root = TestRoot::spawn();
    let spawned = Arc::new(AtomicU32::new(u32::default()));
    let result = scanner()
        .with_behavior(super::super::process_snapshot_fault::REPORT_FAILURE)
        .with_spawn_observer(Arc::clone(&spawned))
        .scan(root.identity, deadline())
        .await;

    assert!(result.is_err());
    super::test_support::await_child_reaped(observed_process(&spawned)).await;
    assert!(scanner().scan(root.identity, deadline()).await.is_ok());
}

#[tokio::test]
async fn helper_exit_before_frame_is_reaped_and_does_not_poison_retry() {
    let root = TestRoot::spawn();
    let spawned = Arc::new(AtomicU32::new(u32::default()));
    let result = scanner()
        .with_behavior(super::super::process_snapshot_fault::FAIL_BEFORE_FRAME)
        .with_spawn_observer(Arc::clone(&spawned))
        .scan(root.identity, deadline())
        .await;

    assert!(result.is_err());
    super::test_support::await_child_reaped(observed_process(&spawned)).await;
    assert!(scanner().scan(root.identity, deadline()).await.is_ok());
}

#[tokio::test]
async fn helper_parent_watch_exits_when_the_parent_endpoint_closes() {
    let root = TestRoot::spawn();
    let (parent_socket, child_socket) = std::os::unix::net::UnixStream::pair().unwrap();
    let mut command =
        super::command::prepare(&runtime_executable(), root.identity, child_socket, &[]).unwrap();
    command.env(
        super::super::process_snapshot_fault::BEHAVIOR_ENV,
        super::super::process_snapshot_fault::HANG_BEFORE_FRAME,
    );
    let child = ScopedWaitableChild::spawn_before_deadline(&mut command, deadline()).unwrap();
    let process_id = child.process_id();
    drop(command);

    drop(parent_socket);
    assert!(child.wait_for_clean_exit(deadline()).await.is_err());
    super::test_support::await_child_reaped(process_id).await;
}

fn observed_process(observer: &AtomicU32) -> u32 {
    let process_id = observer.load(Ordering::Acquire);
    assert_ne!(process_id, u32::default(), "{HELPER_PROCESS_MISSING}");
    process_id
}
