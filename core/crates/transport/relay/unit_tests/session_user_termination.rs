use super::*;
use crate::model::ExitReason;
use crate::protocol::{error_code, method};
use crate::relay::session_test_support::{completed_session, host_status};
use crate::relay::test_support::{FakeHost, Reply};

#[tokio::test]
async fn disconnected_ending_relay_stops_when_the_worker_was_explicitly_terminated() {
    let terminal_id = Uuid::new_v4();
    let mut stopped = completed_session(terminal_id);
    stopped.completed_attachment = None;
    stopped
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .reason = ExitReason::UserTerminate;
    let host = FakeHost::start(vec![
        Reply::retryable_error(error_code::WORKER_UNREACHABLE),
        Reply::value(&host_status()),
        Reply::value(&stopped),
    ])
    .await;
    let runtime = RelayRuntime::existing(host.client());
    let result = attach_with_recovery(
        &runtime,
        terminal_id,
        u64::default(),
        AttachmentRecovery::TransportOnly,
        &RelayCancellation::default(),
    )
    .await;
    assert!(
        matches!(result, Err(RelayError::SessionNotAttachable(actual)) if actual == terminal_id)
    );
    assert_eq!(
        host.methods(),
        [
            method::ATTACH_SESSION,
            method::HOST_STATUS,
            method::GET_SESSION
        ]
    );
}

#[tokio::test]
async fn disconnected_active_relay_does_not_resume_a_user_terminated_provider() {
    let terminal_id = Uuid::new_v4();
    let mut stopped = completed_session(terminal_id);
    stopped.completed_attachment = None;
    stopped
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .reason = ExitReason::UserTerminate;
    let host = FakeHost::start(vec![Reply::value(&host_status()), Reply::value(&stopped)]).await;
    let result = RelayRuntime::existing(host.client())
        .reconnect(terminal_id)
        .await;
    assert!(
        matches!(result, Err(RelayError::SessionNotAttachable(actual)) if actual == terminal_id)
    );
    assert_eq!(host.methods(), [method::HOST_STATUS, method::GET_SESSION]);
}
