use super::*;
use crate::model::FaultReapProgressKind;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

const ESCROW_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const REPEATED_OWNER_SPAWN_FAILURES: usize = 3;
const STRESS_SLOT_COUNT: usize = 1_024;
const INJECTED_DROP_PANIC: &str = "injected repeated-spawn emergency drop panic";

#[test]
fn repeated_owner_spawn_failures_preserve_custody_and_complete_once() {
    let observer = FaultReapStateObserver::default();
    let mut producer = EmergencyEscrowProducer::reserve(observer.clone()).unwrap();
    let probe = producer.probe_for_test();
    probe.fail_owner_spawns(REPEATED_OWNER_SPAWN_FAILURES);
    let (custody, root) = fixture::live_custody();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    producer.completion = Some(completion);

    producer.install_after_panic(custody, Box::new(INJECTED_DROP_PANIC));

    probe.wait_for_owner_spawn_failures(REPEATED_OWNER_SPAWN_FAILURES);
    let outcome = receiver.receive_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    probe.wait_for_slot_drop();
    assert!(!root.matches());
    assert!(matches!(
        receiver.receive_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Disconnected)
    ));
    let status = observer.status().unwrap();
    assert!(!status.custody_retained);
    assert_eq!(
        status.last_semantic_progress.unwrap().kind,
        FaultReapProgressKind::Completed
    );
    assert!(
        status
            .last_fault
            .is_some_and(|detail| detail.contains(INJECTED_OWNER_SPAWN_FAILURE_DETAIL))
    );
}

#[test]
fn concurrent_scale_slots_cancel_and_prune_without_retention() {
    let mut producers = Vec::with_capacity(STRESS_SLOT_COUNT);
    let mut probes = Vec::with_capacity(STRESS_SLOT_COUNT);
    for _ in usize::default()..STRESS_SLOT_COUNT {
        let producer = EmergencyEscrowProducer::reserve(FaultReapStateObserver::default()).unwrap();
        probes.push(producer.probe_for_test());
        producers.push(producer);
    }

    drop(producers);

    for probe in probes {
        probe.wait_for_slot_drop();
        assert!(probe.phase().is_none());
    }
}
