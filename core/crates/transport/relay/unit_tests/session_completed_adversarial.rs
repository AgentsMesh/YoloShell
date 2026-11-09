use super::*;
use crate::model::ExitRecord;
use crate::protocol::{EventEnvelope, ProtocolError, event, method};
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, completed_session, empty_attach, empty_attach_for,
    exit_event, host_status, natural_exit, options, session_event,
};
use crate::relay::test_support::{FakeHost, Reply};
use std::time::Duration;
use tokio::sync::mpsc;

const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const DIFFERENT_EXIT_CODE: i32 = 23;

#[tokio::test]
async fn completed_runtime_rejects_replacement_epoch_before_ending() {
    let terminal_id = Uuid::new_v4();
    let replacement_epoch = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(empty_attach_for(replacement_epoch), Vec::new()),
    ])
    .await;
    let (status, mut statuses) = mpsc::unbounded_channel();

    let error = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
        status,
    )
    .await
    .unwrap_err();

    assert!(
        matches!(error, RelayError::RuntimeEpochChanged { actual, .. } if actual == replacement_epoch)
    );
    assert!(statuses.try_recv().is_err());
    assert_no_control_requests(&host);
}

#[tokio::test]
async fn completed_runtime_rejects_exit_that_differs_from_attestation() {
    let terminal_id = Uuid::new_v4();
    let mut forged_exit = natural_exit();
    forged_exit.code = Some(DIFFERENT_EXIT_CODE);
    let forged_event = completed_exit_event(terminal_id, &forged_exit);
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(empty_attach(), vec![forged_event]),
    ])
    .await;
    let (status, mut statuses) = mpsc::unbounded_channel();

    let error = relay_session_with_status(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
        status,
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        RelayError::Protocol(ProtocolError::EventMismatch)
    ));
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert!(statuses.try_recv().is_err());
    assert_no_control_requests(&host);
}

#[tokio::test]
async fn completed_disconnect_revalidates_attestation_without_session_recovery() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::closing_attachment(empty_attach()),
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(empty_attach(), vec![exit_event(terminal_id)]),
    ])
    .await;

    let exit = tokio::time::timeout(
        TEST_TIMEOUT,
        relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        ),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(exit, RelayExit::SessionExited);
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
        ]
    );
    assert_no_control_requests(&host);
}

#[tokio::test]
async fn completed_output_gap_revalidates_attestation_before_reattach() {
    let terminal_id = Uuid::new_v4();
    let host = FakeHost::start(vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(
            empty_attach(),
            vec![session_event(event::OUTPUT_GAP, terminal_id)],
        ),
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
        Reply::attachment(empty_attach(), vec![exit_event(terminal_id)]),
    ])
    .await;

    let exit = tokio::time::timeout(
        TEST_TIMEOUT,
        relay_session(
            RelayRuntime::existing(host.client()),
            options(terminal_id),
            PendingInput,
            tokio::io::sink(),
            closed_resize_channel(),
        ),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(exit, RelayExit::SessionExited);
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
        ]
    );
    assert_no_control_requests(&host);
}

fn completed_exit_event(terminal_id: Uuid, exit: &ExitRecord) -> EventEnvelope {
    EventEnvelope::new(
        event::SESSION_EXITED,
        Some(terminal_id.to_string()),
        Some(exit.runtime_epoch.to_string()),
        exit,
    )
    .unwrap()
}

fn assert_no_control_requests(host: &FakeHost) {
    assert!(!host.methods().iter().any(|name| matches!(
        name.as_str(),
        method::RESUME_SESSION_EXACT
            | method::ACQUIRE_WRITER
            | method::GET_WRITER_LEASE
            | method::TAKEOVER_WRITER
            | method::RENEW_WRITER
            | method::RELEASE_WRITER
            | method::TERMINAL_INPUT
            | method::RESIZE_TERMINAL
            | method::SIGNAL_SESSION
    )));
}
