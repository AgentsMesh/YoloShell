use super::{FaultReaperFallback, recover_after_spawn_failure_for_test, spawn_owner_with};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, sync_channel};
use std::time::Duration;

const SIGNAL_CHANNEL_CAPACITY: usize = 1;
const EXECUTION_INCREMENT: usize = 1;
const EXPECTED_EXECUTIONS: usize = 1;
const FALLBACK_TEST_TIMEOUT: Duration = Duration::from_secs(2);
const BLOCKED_OWNER_START_TIMEOUT_DETAIL: &str = "blocked fallback owner did not start";
const LATER_OWNER_START_TIMEOUT_DETAIL: &str = "later fallback owner was starved";
const BLOCKED_OWNER_RELEASE_FAILURE_DETAIL: &str = "blocked fallback owner stopped early";
const SPAWN_PAYLOAD_DROP_PANIC: &str = "injected fallback spawn payload Drop panic";

struct SpawnPanicPayload(Arc<AtomicUsize>);

impl Drop for SpawnPanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(EXECUTION_INCREMENT, Ordering::AcqRel);
        panic!("{SPAWN_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn blocked_owner_does_not_starve_later_fallback_task() {
    let fallback = FaultReaperFallback::capture().unwrap();
    let (started_sender, started_receiver) = sync_channel(SIGNAL_CHANNEL_CAPACITY);
    let (release_sender, release_receiver) = channel();
    fallback.handoff_task_for_test(move || {
        let _ = started_sender.send(());
        let _ = release_receiver.recv();
    });
    started_receiver
        .recv_timeout(FALLBACK_TEST_TIMEOUT)
        .expect(BLOCKED_OWNER_START_TIMEOUT_DETAIL);
    let (later_sender, later_receiver) = sync_channel(SIGNAL_CHANNEL_CAPACITY);

    fallback.handoff_task_for_test(move || {
        let _ = later_sender.send(());
    });

    later_receiver
        .recv_timeout(FALLBACK_TEST_TIMEOUT)
        .expect(LATER_OWNER_START_TIMEOUT_DETAIL);
    release_sender
        .send(())
        .expect(BLOCKED_OWNER_RELEASE_FAILURE_DETAIL);
}

#[test]
fn owner_spawn_failure_returns_unclaimed_task_for_retry() {
    let executions = Arc::new(AtomicUsize::new(usize::default()));
    let observed = Arc::clone(&executions);

    let recovered = recover_after_spawn_failure_for_test(move || {
        observed.fetch_add(EXECUTION_INCREMENT, Ordering::AcqRel);
    });

    assert_eq!(executions.load(Ordering::Acquire), usize::default());
    recovered();
    assert_eq!(executions.load(Ordering::Acquire), EXPECTED_EXECUTIONS);
}

#[test]
fn owner_spawn_panic_payload_drop_returns_unclaimed_task_for_retry() {
    let executions = Arc::new(AtomicUsize::new(usize::default()));
    let observed = Arc::clone(&executions);
    let payload_drops = Arc::new(AtomicUsize::new(usize::default()));
    let failure = spawn_owner_with(
        Box::new(move || {
            observed.fetch_add(EXECUTION_INCREMENT, Ordering::AcqRel);
        }),
        |_| std::panic::panic_any(SpawnPanicPayload(Arc::clone(&payload_drops))),
    )
    .unwrap_err();

    assert_eq!(executions.load(Ordering::Acquire), usize::default());
    assert_eq!(payload_drops.load(Ordering::Acquire), EXPECTED_EXECUTIONS);
    (failure.task)();
    assert_eq!(executions.load(Ordering::Acquire), EXPECTED_EXECUTIONS);
}
