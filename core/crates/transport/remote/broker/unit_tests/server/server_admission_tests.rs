use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const ADOPTION_PAYLOAD_DROP_PANIC: &str = "injected adoption payload Drop panic";

struct AdoptionPayload(Arc<AtomicBool>);

impl Drop for AdoptionPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{ADOPTION_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn pending_admission_allows_only_observation_and_bootstrap() {
    assert!(permits_pending_request(&BrokerRequest::Status));
    assert!(permits_pending_request(&BrokerRequest::Bootstrap));
    assert!(permits_pending_request(
        &BrokerRequest::PrepareHubTransport(yolo_broker_hub_test_support::profile())
    ));
    assert!(!permits_pending_request(&BrokerRequest::OpenHubPeer(
        yolo_broker_hub_test_support::request()
    )));
    assert!(!permits_pending_request(&BrokerRequest::WorkingDirectory));
    assert!(!permits_pending_request(&BrokerRequest::Shutdown {
        expected_revision: String::new(),
        expected_instance_id: Uuid::nil(),
    }));
}

#[test]
fn admission_is_observable_as_pending_while_ack_delivery_runs() {
    let admission = Arc::new(BrokerAdmission::new(true));
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let adopting = Arc::clone(&admission);
    let adoption = std::thread::spawn(move || {
        adopting.adopt(
            Uuid::new_v4(),
            Box::new(move |_| {
                entered_send.send(()).unwrap();
                release_receive.recv().unwrap();
                true
            }),
        );
    });
    entered_receive.recv().unwrap();

    let observing = Arc::clone(&admission);
    let (observed_send, observed_receive) = std::sync::mpsc::channel();
    let observation = std::thread::spawn(move || {
        observed_send.send(observing.is_adopted()).unwrap();
    });
    assert!(!observed_receive.recv().unwrap());
    release_send.send(()).unwrap();
    adoption.join().unwrap();
    assert!(admission.is_adopted());
    observation.join().unwrap();
}

#[test]
fn closure_during_ack_delivery_cannot_reopen_admission() {
    let admission = Arc::new(BrokerAdmission::new(true));
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let adopting = Arc::clone(&admission);
    let adoption = std::thread::spawn(move || {
        adopting.adopt(
            Uuid::new_v4(),
            Box::new(move |_| {
                entered_send.send(()).unwrap();
                release_receive.recv().unwrap();
                true
            }),
        );
    });
    entered_receive.recv().unwrap();
    admission.close();
    release_send.send(()).unwrap();
    adoption.join().unwrap();
    assert!(!admission.is_open());
    assert!(!admission.is_adopted());
}

#[test]
fn failed_ack_closes_admission() {
    let admission = BrokerAdmission::new(true);
    admission.adopt(Uuid::new_v4(), Box::new(|_| false));
    assert!(!admission.is_adopted());
    assert!(!admission.is_open());
}

#[test]
fn panicking_ack_payload_drop_is_contained_and_closes_admission() {
    let admission = BrokerAdmission::new(true);
    let payload_dropped = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&payload_dropped);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        admission.adopt(
            Uuid::new_v4(),
            Box::new(move |_| std::panic::panic_any(AdoptionPayload(observed))),
        );
    }));

    assert!(outcome.is_ok());
    assert!(payload_dropped.load(Ordering::SeqCst));
    assert!(!admission.is_open());
    assert!(!admission.is_adopted());
}
