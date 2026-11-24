use super::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ESCROW_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const INJECTED_DROP_PANIC: &str = "injected emergency escrow regression drop panic";
const INSTALLED_SLOT_BACKGROUND_REFERENCES: usize = 2;
const CONTROL_PLANE_MISSING_DETAIL: &str = "emergency escrow control plane is reserved";

#[test]
fn install_moves_the_producer_reference_before_returning() {
    let slots = Arc::new(Mutex::new(Vec::new()));
    let registry = local_registry(&slots);
    let mut producer =
        EmergencyEscrowProducer::reserve_in_for_test(FaultReapStateObserver::default(), registry);
    let probe = producer.probe_for_test();
    probe.block_install_return();
    let (custody, root) = fixture::live_custody();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    producer.completion = Some(completion);

    let installer = std::thread::spawn(move || {
        producer.install_after_panic(custody, Box::new(INJECTED_DROP_PANIC));
    });
    probe.wait_for_install_publication();
    let references_before_return = probe.strong_count();
    probe.allow_install_return();
    installer.join().unwrap();

    assert_eq!(
        references_before_return,
        INSTALLED_SLOT_BACKGROUND_REFERENCES
    );
    settle_and_prune(&slots, &probe, &receiver);
    assert!(!root.matches());
}

#[test]
fn owner_drive_refreshes_an_expired_reserved_schedule() {
    let slots = Arc::new(Mutex::new(Vec::new()));
    let registry = local_registry(&slots);
    let mut producer =
        EmergencyEscrowProducer::reserve_in_for_test(FaultReapStateObserver::default(), registry);
    let probe = producer.probe_for_test();
    producer
        .control
        .as_mut()
        .expect(CONTROL_PLANE_MISSING_DETAIL)
        .schedule = super::super::schedule::FaultReapSchedule::expired_for_test();
    let (custody, root) = fixture::live_custody();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    producer.completion = Some(completion);

    producer.install_after_panic(custody, Box::new(INJECTED_DROP_PANIC));
    runtime::dispatch_ready(&slots);

    assert!(probe.wait_for_drive_schedule());
    settle_and_prune(&slots, &probe, &receiver);
    assert!(!root.matches());
}

fn local_registry(slots: &Arc<Mutex<Vec<Arc<EmergencySlot>>>>) -> EmergencyRegistry {
    EmergencyRegistry {
        slots: Arc::clone(slots),
        dispatcher: std::thread::current(),
    }
}

fn settle_and_prune(
    slots: &Mutex<Vec<Arc<EmergencySlot>>>,
    probe: &test_support::EmergencyEscrowProbe,
    receiver: &super::super::FaultReapReceiver,
) {
    runtime::dispatch_ready(slots);
    receiver.receive_timeout(ESCROW_TEST_TIMEOUT).unwrap();
    let deadline = Instant::now() + ESCROW_TEST_TIMEOUT;
    while probe.phase().is_some() {
        runtime::dispatch_ready(slots);
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    probe.wait_for_slot_drop();
}
