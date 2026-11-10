use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

const ESCROW_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const INJECTED_DROP_PANIC: &str = "injected emergency escrow drop panic";
const LOCAL_DISPATCH_CHANNEL_CAPACITY: usize = 1;

#[test]
fn cancelled_unused_slot_is_pruned_by_dispatcher() {
    let producer = EmergencyEscrowProducer::reserve(FaultReapStateObserver::default()).unwrap();
    let probe = producer.probe_for_test();
    assert!(matches!(probe.phase(), Some(EmergencyPhase::Vacant)));

    drop(producer);

    probe.wait_for_slot_drop();
    assert!(probe.phase().is_none());
}

#[test]
fn successful_normal_handoff_prunes_unused_emergency_slot() {
    let reaper = FaultReaperReservation::reserve().unwrap();
    let probe = reaper.emergency_probe_for_test();
    let (mut ownership, root) = fixture::live_ownership(reaper);
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    let settlement = ownership.settle_after_fault(completion);

    assert!(matches!(
        settlement,
        crate::worker::pty_ownership::FaultSettlement::Deferred(_)
    ));
    let outcome = receiver.receive_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    probe.wait_for_slot_drop();
    assert!(probe.phase().is_none());
    assert!(!root.matches());
}

#[test]
fn unpark_between_empty_scan_and_park_drives_new_ready_slot() {
    let slots = Arc::new(Mutex::new(Vec::new()));
    let dispatch_slots = Arc::clone(&slots);
    let proceed_to_park = Arc::new(AtomicBool::new(false));
    let dispatch_proceed = Arc::clone(&proceed_to_park);
    let (thread_sender, thread_receiver) = mpsc::sync_channel(LOCAL_DISPATCH_CHANNEL_CAPACITY);
    let (scanned_sender, scanned_receiver) = mpsc::sync_channel(LOCAL_DISPATCH_CHANNEL_CAPACITY);
    let (done_sender, done_receiver) = mpsc::sync_channel(LOCAL_DISPATCH_CHANNEL_CAPACITY);
    let dispatcher = std::thread::spawn(move || {
        thread_sender.send(std::thread::current()).unwrap();
        assert!(!runtime::dispatch_ready(&dispatch_slots));
        scanned_sender.send(()).unwrap();
        while !dispatch_proceed.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        std::thread::park();
        runtime::dispatch_ready(&dispatch_slots);
        done_sender.send(()).unwrap();
    });
    let dispatcher_thread = thread_receiver.recv_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    scanned_receiver.recv_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    let registry = EmergencyRegistry {
        slots: Arc::clone(&slots),
        dispatcher: dispatcher_thread,
    };
    let mut producer =
        EmergencyEscrowProducer::reserve_in_for_test(FaultReapStateObserver::default(), registry);
    let (custody, root) = fixture::live_custody();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    producer.completion = Some(completion);

    producer.install_after_panic(custody, Box::new(INJECTED_DROP_PANIC));
    proceed_to_park.store(true, Ordering::Release);

    done_receiver.recv_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    dispatcher.join().unwrap();
    let outcome = receiver.receive_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
    assert!(!root.matches());
    runtime::dispatch_ready(&slots);
}
