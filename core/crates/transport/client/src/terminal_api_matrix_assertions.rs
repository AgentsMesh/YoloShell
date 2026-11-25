use super::*;
use crate::protocol::method;

pub(super) struct TerminalExpectations<'a> {
    pub(super) target: RuntimeRef,
    pub(super) holder: &'a str,
    pub(super) controller: &'a str,
    pub(super) ttl_ms: u64,
    pub(super) explicit_capability: &'a WriterCapability,
    pub(super) basic_grant: &'a WriterLeaseGrant,
    pub(super) takeover_grant: &'a WriterLeaseGrant,
    pub(super) generated_input_id: Uuid,
    pub(super) explicit_input: &'a InputRequest,
    pub(super) input: &'a [u8],
    pub(super) cols: u16,
    pub(super) rows: u16,
    pub(super) signal: &'a str,
}

pub(super) fn assert_terminal_requests(
    mut requests: api_matrix_support::MatrixRequests,
    expected: TerminalExpectations<'_>,
) {
    let instance_acquire = requests
        .take_by_typed_key::<LeaseRequest, _>(method::ACQUIRE_WRITER, |request| {
            request.controller_instance_id.as_deref() == Some(expected.controller)
        });
    assert_lease(
        &instance_acquire,
        &expected,
        None,
        Some(expected.controller),
        expected.explicit_capability,
    );
    let basic_acquire = requests.take_unique::<LeaseRequest>(method::ACQUIRE_WRITER);
    assert_lease(
        &basic_acquire,
        &expected,
        None,
        None,
        &expected.basic_grant.writer_capability,
    );

    let instance_takeover = requests
        .take_by_typed_key::<LeaseRequest, _>(method::TAKEOVER_WRITER, |request| {
            request.controller_instance_id.as_deref() == Some(expected.controller)
        });
    assert_lease(
        &instance_takeover,
        &expected,
        Some(expected.basic_grant.generation),
        Some(expected.controller),
        expected.explicit_capability,
    );
    let basic_takeover = requests.take_unique::<LeaseRequest>(method::TAKEOVER_WRITER);
    assert_lease(
        &basic_takeover,
        &expected,
        None,
        None,
        &expected.takeover_grant.writer_capability,
    );

    let renew = requests.take_unique::<LeaseRequest>(method::RENEW_WRITER);
    assert_lease_fields(
        &renew,
        expected.target,
        &expected.basic_grant.holder_id,
        Some(expected.ttl_ms),
        Some(expected.basic_grant.generation),
        expected.basic_grant.controller_instance_id.as_deref(),
        &expected.basic_grant.writer_capability,
    );
    let release = requests.take_unique::<LeaseRequest>(method::RELEASE_WRITER);
    assert_lease_fields(
        &release,
        expected.target,
        &expected.basic_grant.holder_id,
        None,
        Some(expected.basic_grant.generation),
        expected.basic_grant.controller_instance_id.as_deref(),
        &expected.basic_grant.writer_capability,
    );
    let lookup = requests.take_unique::<RuntimeRef>(method::GET_WRITER_LEASE);
    assert_eq!(lookup, expected.target);

    let explicit_input = requests
        .take_by_typed_key::<InputRequest, _>(method::TERMINAL_INPUT, |request| {
            request.input_id == expected.explicit_input.input_id
        });
    assert_input(&explicit_input, expected.explicit_input);
    let generated_input = requests.take_unique::<InputRequest>(method::TERMINAL_INPUT);
    assert_eq!(generated_input.input_id, expected.generated_input_id);
    assert_input_fields(&generated_input, &expected);

    let resize = requests.take_unique::<ResizeRequest>(method::RESIZE_TERMINAL);
    assert_runtime_fence(
        resize.terminal_id,
        resize.runtime_epoch,
        resize.generation,
        &resize.writer_capability,
        &expected,
    );
    assert_eq!((resize.cols, resize.rows), (expected.cols, expected.rows));
    let signal = requests.take_unique::<SignalRequest>(method::SIGNAL_SESSION);
    assert_runtime_fence(
        signal.terminal_id,
        signal.runtime_epoch,
        signal.generation,
        &signal.writer_capability,
        &expected,
    );
    assert_eq!(signal.signal, expected.signal);

    let get = requests.take_unique::<TerminalRef>(method::GET_SESSION);
    assert_eq!(get.terminal_id, expected.target.terminal_id);
    let snapshot = requests.take_unique::<SnapshotRequest>(method::GET_SNAPSHOT);
    assert_eq!(snapshot.terminal_id, expected.target.terminal_id);
    assert_eq!(snapshot.runtime_epoch, expected.target.runtime_epoch);
    requests.assert_empty();
}

fn assert_lease(
    request: &LeaseRequest,
    expected: &TerminalExpectations<'_>,
    generation: Option<u64>,
    controller: Option<&str>,
    capability: &WriterCapability,
) {
    assert_lease_fields(
        request,
        expected.target,
        expected.holder,
        Some(expected.ttl_ms),
        generation,
        controller,
        capability,
    );
}

fn assert_lease_fields(
    request: &LeaseRequest,
    target: RuntimeRef,
    holder: &str,
    ttl_ms: Option<u64>,
    generation: Option<u64>,
    controller: Option<&str>,
    capability: &WriterCapability,
) {
    assert_eq!(request.terminal_id, target.terminal_id);
    assert_eq!(request.runtime_epoch, target.runtime_epoch);
    assert_eq!(request.holder_id, holder);
    assert_eq!(request.ttl_ms, ttl_ms);
    assert_eq!(request.generation, generation);
    assert_eq!(request.controller_instance_id.as_deref(), controller);
    assert_eq!(&request.writer_capability, capability);
}

fn assert_input(actual: &InputRequest, expected: &InputRequest) {
    assert_eq!(actual.terminal_id, expected.terminal_id);
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.generation, expected.generation);
    assert_eq!(actual.writer_capability, expected.writer_capability);
    assert_eq!(actual.input_id, expected.input_id);
    assert_eq!(actual.data, expected.data);
}

fn assert_input_fields(input: &InputRequest, expected: &TerminalExpectations<'_>) {
    assert_runtime_fence(
        input.terminal_id,
        input.runtime_epoch,
        input.generation,
        &input.writer_capability,
        expected,
    );
    assert_eq!(input.data, expected.input);
    assert_ne!(input.input_id, Uuid::nil());
}

fn assert_runtime_fence(
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    generation: u64,
    capability: &WriterCapability,
    expected: &TerminalExpectations<'_>,
) {
    assert_eq!(terminal_id, expected.target.terminal_id);
    assert_eq!(runtime_epoch, expected.target.runtime_epoch);
    assert_eq!(generation, expected.basic_grant.generation);
    assert_eq!(capability, &expected.basic_grant.writer_capability);
}
