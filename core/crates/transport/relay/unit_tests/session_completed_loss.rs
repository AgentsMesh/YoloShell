use super::*;
use crate::protocol::method;
use crate::relay::session_test_support::{
    PendingInput, closed_resize_channel, completed_session, empty_attach, host_status, options,
    running_session,
};
use crate::relay::test_support::{FakeHost, Reply};

const REPLACEMENT_EXIT_CODE: i32 = 91;

#[tokio::test]
async fn completed_worker_loss_before_attach_terminates_without_resume() {
    let terminal_id = Uuid::new_v4();
    let replies = completed_prefix(terminal_id)
        .into_iter()
        .chain([
            Reply::retryable_error(crate::protocol::error_code::WORKER_UNREACHABLE),
            Reply::value(&host_status()),
            Reply::value(&running_session(terminal_id)),
        ])
        .collect();
    assert_terminal_worker_loss(terminal_id, replies).await;
}

#[tokio::test]
async fn completed_worker_loss_after_disconnect_terminates_without_resume() {
    let terminal_id = Uuid::new_v4();
    let replies = completed_prefix(terminal_id)
        .into_iter()
        .chain([
            Reply::closing_attachment(empty_attach()),
            Reply::value(&host_status()),
            Reply::value(&running_session(terminal_id)),
        ])
        .collect();
    assert_terminal_worker_loss(terminal_id, replies).await;
}

#[tokio::test]
async fn completed_reconnect_rejects_replacement_epoch() {
    let terminal_id = Uuid::new_v4();
    let mut replacement = completed_session(terminal_id);
    let replacement_epoch = Uuid::new_v4();
    let runtime = replacement.runtime.as_mut().unwrap();
    runtime.runtime_epoch = replacement_epoch;
    runtime.exit.as_mut().unwrap().runtime_epoch = replacement_epoch;
    replacement
        .completed_attachment
        .as_mut()
        .unwrap()
        .runtime_epoch = replacement_epoch;
    assert_replaced_attestation(terminal_id, replacement).await;
}

#[tokio::test]
async fn completed_reconnect_rejects_changed_exit_attestation() {
    let terminal_id = Uuid::new_v4();
    let mut replacement = completed_session(terminal_id);
    replacement
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .code = Some(REPLACEMENT_EXIT_CODE);
    assert_replaced_attestation(terminal_id, replacement).await;
}

fn completed_prefix(terminal_id: Uuid) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&completed_session(terminal_id)),
    ]
}

async fn assert_replaced_attestation(terminal_id: Uuid, replacement: crate::model::TerminalView) {
    let replies = completed_prefix(terminal_id)
        .into_iter()
        .chain([
            Reply::closing_attachment(empty_attach()),
            Reply::value(&host_status()),
            Reply::value(&replacement),
        ])
        .collect();
    assert_terminal_worker_loss(terminal_id, replies).await;
}

async fn assert_terminal_worker_loss(terminal_id: Uuid, replies: Vec<Reply>) {
    let host = FakeHost::start(replies).await;
    let error = relay_session(
        RelayRuntime::existing(host.client()),
        options(terminal_id),
        PendingInput,
        tokio::io::sink(),
        closed_resize_channel(),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, RelayError::SessionNotAttachable(actual) if actual == terminal_id));
    assert_eq!(
        host.methods(),
        [
            method::HOST_STATUS,
            method::GET_SESSION,
            method::ATTACH_SESSION,
            method::HOST_STATUS,
            method::GET_SESSION,
        ]
    );
    assert!(
        !host
            .methods()
            .iter()
            .any(|name| matches!(name.as_str(), method::RESUME_SESSION_EXACT))
    );
}
