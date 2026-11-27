use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

const ESCROW_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const PAYLOAD_DROP_PANIC: &str = "injected emergency escrow payload Drop panic";

enum SpawnFault {
    IoError,
    Panic,
}

struct PanicDropPayload(Arc<AtomicBool>);

impl Drop for PanicDropPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn owner_spawn_io_error_restores_ready_and_preserves_custody() {
    assert_spawn_fault_recovers(SpawnFault::IoError, OWNER_SPAWN_FAILURE_DETAIL);
}

#[test]
fn owner_spawn_panic_restores_ready_and_preserves_custody() {
    assert_spawn_fault_recovers(SpawnFault::Panic, OWNER_SPAWN_PANIC_DETAIL);
}

fn assert_spawn_fault_recovers(fault: SpawnFault, expected_fault: &str) {
    let mut producer = EmergencyEscrowProducer::reserve(FaultReapStateObserver::default()).unwrap();
    let probe = producer.probe_for_test();
    match fault {
        SpawnFault::IoError => probe.fail_next_owner_spawn(),
        SpawnFault::Panic => probe.panic_next_owner_spawn(),
    }
    let (custody, root) = fixture::live_custody();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    producer.completion = Some(completion);
    let payload_dropped = Arc::new(AtomicBool::new(false));

    producer.install_after_panic(
        custody,
        Box::new(PanicDropPayload(Arc::clone(&payload_dropped))),
    );
    probe.wait_for_owner_spawn_failure();

    assert!(matches!(probe.phase(), Some(EmergencyPhase::Ready)));
    assert!(root.matches());
    assert!(!payload_dropped.load(Ordering::SeqCst));
    probe.allow_owner_spawn_retry();
    let outcome = receiver.receive_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    assert!(
        outcome
            .fault
            .as_ref()
            .is_some_and(|error| error.to_string().contains(expected_fault))
    );
    assert!(payload_dropped.load(Ordering::SeqCst));
    probe.wait_for_slot_drop();
    assert!(matches!(
        receiver.receive_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Disconnected)
    ));
    assert!(!root.matches());
}
