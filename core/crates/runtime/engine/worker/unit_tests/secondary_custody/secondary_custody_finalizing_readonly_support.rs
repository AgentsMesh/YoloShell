use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const FINALIZING_LEASE_HOLDER: &str = "finalizing-read-only";
#[cfg(target_os = "macos")]
const FINALIZING_SIGNAL: &str = "CONT";
#[cfg(target_os = "macos")]
const FINALIZING_COLUMNS: u16 = 100;
#[cfg(target_os = "macos")]
const FINALIZING_ROWS: u16 = 40;
#[cfg(target_os = "macos")]
const FINALIZING_INPUT: &[u8] = b"finalizing-read-only-input";
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_OUTPUT: &[u8] = b"finalizing-read-only-output";
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_EXIT_CODE: i32 = 0;
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_ATTACH_AFTER_SEQUENCE: u64 = yolo_relay::INITIAL_OUTPUT_SEQUENCE;
#[cfg(target_os = "macos")]
const FINALIZING_NONFORCED: bool = false;
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_EXIT_REASON: ExitReason = ExitReason::Natural;
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_NO_SIGNAL: Option<i32> = None;
#[cfg(target_os = "macos")]
pub(super) const FINALIZING_EXPECTED_STATUS: TerminalRuntimeStatus = TerminalRuntimeStatus::Finalizing;

#[cfg(target_os = "macos")]
pub(super) async fn assert_not_running_finalizing(fixture: &Fixture) {
    let runtime = RuntimeRef::new(fixture.terminal_id(), fixture.state.runtime_epoch());
    let generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let capability = WriterCapability::generate();
    let mut lease = fixture.lease_request(FINALIZING_LEASE_HOLDER);
    lease.generation = Some(generation);
    lease.writer_capability = capability.clone();
    let input = InputRequest {
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        generation,
        writer_capability: capability.clone(),
        input_id: Uuid::new_v4(),
        data: FINALIZING_INPUT.to_vec(),
    };
    let resize = ResizeRequest {
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        generation,
        writer_capability: capability.clone(),
        cols: FINALIZING_COLUMNS,
        rows: FINALIZING_ROWS,
    };
    let signal = SignalRequest {
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        generation,
        writer_capability: capability,
        signal: FINALIZING_SIGNAL.into(),
    };
    for (method_name, body) in [
        (method::ACQUIRE_WRITER, to_value(&lease).unwrap()),
        (method::GET_WRITER_LEASE, to_value(&runtime).unwrap()),
        (method::TAKEOVER_WRITER, to_value(&lease).unwrap()),
        (method::RENEW_WRITER, to_value(&lease).unwrap()),
        (method::RELEASE_WRITER, to_value(&lease).unwrap()),
        (method::TERMINAL_INPUT, to_value(&input).unwrap()),
        (method::RESIZE_TERMINAL, to_value(&resize).unwrap()),
        (method::SIGNAL_SESSION, to_value(&signal).unwrap()),
    ] {
        let request = RequestEnvelope::new(Uuid::new_v4().to_string(), method_name, body);
        let error = dispatch_request_inner(
            &request,
            &fixture.state,
            fixture.state.current_authenticated_controller(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, lease_error_code::SESSION_NOT_RUNNING);
    }
    let snapshot = SnapshotRequest {
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
    };
    let error = dispatch_test(&fixture.state, method::GET_SNAPSHOT, &snapshot)
        .await
        .unwrap_err();
    assert_eq!(error.code, lease_error_code::SESSION_NOT_RUNNING);
    let exact = FencedTerminateRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        expected_status: Some(FINALIZING_EXPECTED_STATUS),
        force: FINALIZING_NONFORCED,
    };
    assert!(
        dispatch_test(&fixture.state, method::TERMINATE_RUNTIME_EXACT, &exact)
            .await
            .is_err()
    );
    let (mut client, task) = authenticated_worker_connection(fixture, generation).await;
    let attach = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: FINALIZING_ATTACH_AFTER_SEQUENCE,
        })
        .unwrap(),
    );
    let response = client.exchange(&attach).await.unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        lease_error_code::SESSION_NOT_RUNNING
    );
    drop(client);
    task.await.unwrap().unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn pending_termination_rejects_snapshot_and_attachment_before_child_wait() {
    let fixture = Fixture::new();
    let permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);

    let snapshot = SnapshotRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
    };
    let error = dispatch_test(&fixture.state, method::GET_SNAPSHOT, &snapshot)
        .await
        .unwrap_err();
    assert_eq!(error.code, lease_error_code::SESSION_NOT_RUNNING);

    let (mut client, task) = authenticated_worker_connection(
        &fixture,
        fixture.state.controller_generation.load(Ordering::Acquire),
    )
    .await;
    let attach = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: FINALIZING_ATTACH_AFTER_SEQUENCE,
        })
        .unwrap(),
    );
    let response = client.exchange(&attach).await.unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        lease_error_code::SESSION_NOT_RUNNING
    );
    drop(client);
    task.await.unwrap().unwrap();
}
