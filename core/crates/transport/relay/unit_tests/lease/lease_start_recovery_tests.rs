use super::*;
use crate::lease::constants::{DEFAULT_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::protocol::{error_code, method};
use crate::relay::session_test_support::{host_status, running_session};
use crate::relay::test_support::{FakeHost, Reply};
use crate::relay::{RelayCancellation, RelayOptions};
use crate::worker::LeaseRequest;

const HOLDER: &str = "initial-holder";
const FOREIGN_HOLDER: &str = "initial-foreign-holder";
const FOREIGN_CONTROLLER: &str = "initial-foreign-controller";
const FOREIGN_GENERATION: u64 = 61;
const ACQUIRED_GENERATION: u64 = 62;
const TAKEOVER_GENERATION: u64 = 63;

fn controller(host: &FakeHost, terminal_id: Uuid) -> LeaseController {
    let options = RelayOptions::new(terminal_id, HOLDER).unwrap();
    let (status, _events) = tokio::sync::mpsc::unbounded_channel();
    LeaseController::pending(
        RelayRuntime::existing(host.client()),
        &options,
        test_runtime_target(terminal_id),
        status.into(),
    )
}

fn foreign_lease() -> WriterLease {
    WriterLease {
        holder_id: FOREIGN_HOLDER.into(),
        controller_instance_id: Some(FOREIGN_CONTROLLER.into()),
        generation: FOREIGN_GENERATION,
        expires_at_ms: i64::MAX,
        ttl_ms: DEFAULT_TTL_MS,
        renew_after_ms: DEFAULT_TTL_MS / RENEW_AFTER_DIVISOR,
    }
}

fn requests_for(host: &FakeHost, expected_method: &str) -> Vec<LeaseRequest> {
    host.requests()
        .into_iter()
        .filter(|request| request.method == expected_method)
        .map(|request| request.decode_payload().unwrap())
        .collect()
}

fn assert_same_writer_intent(requests: &[LeaseRequest]) {
    let first = requests.first().unwrap();
    assert!(requests.iter().all(|request| {
        request.holder_id == first.holder_id
            && request.controller_instance_id == first.controller_instance_id
            && request.writer_capability == first.writer_capability
    }));
}

#[tokio::test]
async fn initial_acquire_recovers_worker_unreachable_before_retrying_control_intent() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::Lease(ACQUIRED_GENERATION),
    ])
    .await;
    let mut controller = controller(&host, terminal_id);

    controller
        .start(false, &RelayCancellation::default())
        .await
        .unwrap();

    assert_eq!(controller.generation(), Some(ACQUIRED_GENERATION));
    assert_eq!(
        host.methods(),
        [
            method::ACQUIRE_WRITER,
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ACQUIRE_WRITER,
        ]
    );
    let requests = requests_for(&host, method::ACQUIRE_WRITER);
    assert_same_writer_intent(&requests);
    assert!(requests.iter().all(|request| request.generation.is_none()));
}

#[tokio::test]
async fn initial_takeover_recovers_retryable_get_and_mutation_before_retrying_takeover_intent() {
    let terminal_id = Uuid::new_v4();
    let occupied = foreign_lease();
    let host = FakeHost::start(vec![
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(occupied.clone())),
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::value(&Some(occupied)),
        Reply::Lease(TAKEOVER_GENERATION),
    ])
    .await;
    let mut controller = controller(&host, terminal_id);

    controller
        .start(true, &RelayCancellation::default())
        .await
        .unwrap();

    assert_eq!(controller.generation(), Some(TAKEOVER_GENERATION));
    assert_eq!(
        host.methods(),
        [
            method::GET_WRITER_LEASE,
            method::HOST_STATUS,
            method::GET_SESSION,
            method::GET_WRITER_LEASE,
            method::TAKEOVER_WRITER,
            method::HOST_STATUS,
            method::GET_SESSION,
            method::GET_WRITER_LEASE,
            method::TAKEOVER_WRITER,
        ]
    );
    let requests = requests_for(&host, method::TAKEOVER_WRITER);
    assert_same_writer_intent(&requests);
    assert!(
        requests
            .iter()
            .all(|request| request.generation == Some(FOREIGN_GENERATION))
    );
}
