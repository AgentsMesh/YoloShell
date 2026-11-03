use super::*;
use crate::lease::constants::MIN_TTL_MS;
use crate::protocol::{RequestEnvelope, method};
use crate::relay::session_test_support::{
    GENERATION, closed_resize_channel, empty_attach, host_status, options, owned_prefix,
    reconnect_prefix, running_session_for_epoch, test_runtime_epoch,
};
use crate::relay::test_support::{FakeHost, Reply};
use crate::worker::{InputRequest, LeaseRequest, RuntimeRef};
use std::io::Cursor;
use tokio::io::duplex;

const CONTROL_INPUT: &[u8] = b"attachment-epoch-input";
const CONTROL_INPUT_BUFFER_BYTES: usize = 1;
const EXPECTED_REQUEST_COUNT: usize = 1;
const RETRIED_INPUT_REQUEST_COUNT: usize = 2;
const BOUND_REQUEST_MISSING: &str = "expected runtime-bound relay request";
const BOUND_REQUEST_DECODE_FAILED: &str = "decode runtime-bound relay request";
const UNEXPECTED_BOUND_METHOD: &str = "unexpected runtime-bound relay method";

#[tokio::test]
async fn attachment_epoch_targets_acquire_input_and_release() {
    let terminal_id = Uuid::new_v4();
    let expected = RuntimeRef::new(terminal_id, test_runtime_epoch());
    let mut replies = owned_prefix(terminal_id);
    replies.extend([Reply::InputDelivered, Reply::value(&())]);
    let host = FakeHost::start(replies).await;
    let exit = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        Cursor::new(CONTROL_INPUT),
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap();
    assert_eq!(exit, RelayExit::InputClosed);
    assert_bound_targets(
        &host,
        expected,
        &[
            method::ACQUIRE_WRITER,
            method::TERMINAL_INPUT,
            method::RELEASE_WRITER,
        ],
    );
}

#[tokio::test]
async fn maintenance_renew_keeps_the_attachment_epoch() {
    let terminal_id = Uuid::new_v4();
    let expected = RuntimeRef::new(terminal_id, test_runtime_epoch());
    let mut replies = reconnect_prefix(terminal_id);
    replies.extend([
        Reply::attachment(empty_attach(), Vec::new()),
        Reply::LeaseWithTtl {
            generation: GENERATION,
            ttl_ms: MIN_TTL_MS,
        },
        Reply::Lease(GENERATION),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    let (input_sender, input) = duplex(CONTROL_INPUT_BUFFER_BYTES);
    let relay = tokio::spawn(relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        input,
        tokio::io::sink(),
        closed_resize_channel(),
    ));
    host.wait_for_method(method::ACQUIRE_WRITER).await;
    host.wait_for_method(method::RENEW_WRITER).await;
    drop(input_sender);
    assert_eq!(relay.await.unwrap().unwrap(), RelayExit::InputClosed);
    assert_bound_targets(
        &host,
        expected,
        &[
            method::ACQUIRE_WRITER,
            method::RENEW_WRITER,
            method::RELEASE_WRITER,
        ],
    );
}

#[tokio::test]
async fn replacement_seen_during_recovery_never_retargets_old_control() {
    let terminal_id = Uuid::new_v4();
    let attached = RuntimeRef::new(terminal_id, test_runtime_epoch());
    let replacement_epoch = Uuid::new_v4();
    assert_ne!(replacement_epoch, attached.runtime_epoch);
    let mut replies = owned_prefix(terminal_id);
    replies.extend([
        Reply::retryable_error(crate::protocol::error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&running_session_for_epoch(terminal_id, replacement_epoch)),
        Reply::error(crate::protocol::error_code::CONFLICT),
        Reply::value(&()),
    ]);
    let host = FakeHost::start(replies).await;
    assert!(
        relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            Cursor::new(CONTROL_INPUT),
            tokio::io::sink(),
            closed_resize_channel(),
        )
        .await
        .is_err()
    );
    assert_bound_targets(
        &host,
        attached,
        &[method::ACQUIRE_WRITER, method::RELEASE_WRITER],
    );
    let inputs: Vec<_> = host
        .requests()
        .into_iter()
        .filter(|request| request.method == method::TERMINAL_INPUT)
        .map(|request| request.decode_payload::<InputRequest>().unwrap())
        .collect();
    assert_eq!(inputs.len(), RETRIED_INPUT_REQUEST_COUNT);
    let (first, retries) = inputs.split_first().expect(BOUND_REQUEST_MISSING);
    assert_eq!(first.data, CONTROL_INPUT);
    assert!(retries.iter().all(|input| input.input_id == first.input_id));
    assert!(retries.iter().all(|input| input.data == first.data));
    assert!(
        inputs
            .iter()
            .all(|input| { RuntimeRef::new(input.terminal_id, input.runtime_epoch) == attached })
    );
    assert_no_bound_target(&host, replacement_epoch);
}

fn assert_bound_targets(host: &FakeHost, expected: RuntimeRef, methods: &[&str]) {
    let requests = host.requests();
    for method_name in methods {
        let matching: Vec<_> = requests
            .iter()
            .filter(|request| request.method == *method_name)
            .collect();
        assert_eq!(matching.len(), EXPECTED_REQUEST_COUNT);
        assert_target(matching.first().expect(BOUND_REQUEST_MISSING), expected);
    }
}

fn assert_no_bound_target(host: &FakeHost, forbidden_epoch: Uuid) {
    for request in host.requests().iter().filter(|request| {
        matches!(
            request.method.as_str(),
            method::ACQUIRE_WRITER
                | method::RENEW_WRITER
                | method::RELEASE_WRITER
                | method::TERMINAL_INPUT
        )
    }) {
        assert_ne!(request_target(request).runtime_epoch, forbidden_epoch);
    }
}

fn assert_target(request: &RequestEnvelope, expected: RuntimeRef) {
    let actual = request_target(request);
    assert_eq!(actual.terminal_id, expected.terminal_id);
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
}

fn request_target(request: &RequestEnvelope) -> RuntimeRef {
    match request.method.as_str() {
        method::ACQUIRE_WRITER | method::RENEW_WRITER | method::RELEASE_WRITER => {
            let payload: LeaseRequest =
                request.decode_payload().expect(BOUND_REQUEST_DECODE_FAILED);
            RuntimeRef::new(payload.terminal_id, payload.runtime_epoch)
        }
        method::TERMINAL_INPUT => {
            let payload: InputRequest =
                request.decode_payload().expect(BOUND_REQUEST_DECODE_FAILED);
            RuntimeRef::new(payload.terminal_id, payload.runtime_epoch)
        }
        _ => panic!("{UNEXPECTED_BOUND_METHOD}: {}", request.method),
    }
}
