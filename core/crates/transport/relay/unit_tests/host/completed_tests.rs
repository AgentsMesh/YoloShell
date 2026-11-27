use super::*;
use crate::model::{CompletedAttachmentView, ExitReason, TerminalRuntimeStatus};
use crate::protocol::method;
use crate::relay::session_test_support::{completed_session, host_status};
use crate::relay::test_support::{FakeHost, Reply};
use tokio::sync::mpsc;

const MISMATCHED_OUTPUT_SEQUENCE: u64 = 1;

#[test]
fn exact_natural_completion_is_attachment_only() {
    let session = completed_session(Uuid::new_v4());
    let runtime = session.runtime.as_ref().unwrap();
    assert!(matches!(
        session_plan(&session),
        SessionPlan::Attach(PreparedSession::Completed {
            runtime_epoch,
            expected_exit,
        }) if runtime_epoch == runtime.runtime_epoch
            && Some(&expected_exit) == runtime.exit.as_ref()
    ));
}

#[test]
fn natural_completion_without_exact_attestation_never_resumes() {
    let base = completed_session(Uuid::new_v4());
    let mut cases = Vec::new();

    let mut missing = base.clone();
    missing.completed_attachment = None;
    cases.push(missing);

    let mut wrong_epoch = base.clone();
    wrong_epoch.completed_attachment = Some(CompletedAttachmentView {
        runtime_epoch: Uuid::new_v4(),
    });
    cases.push(wrong_epoch);

    let mut wrong_sequence = base;
    wrong_sequence
        .runtime
        .as_mut()
        .unwrap()
        .last_output_sequence = MISMATCHED_OUTPUT_SEQUENCE;
    cases.push(wrong_sequence);

    assert!(
        cases
            .iter()
            .all(|session| session_plan(session) == SessionPlan::PendingFinalization)
    );
}

#[test]
fn exact_non_natural_exit_preserves_provider_recovery() {
    let mut session = completed_session(Uuid::new_v4());
    session.completed_attachment = None;
    session
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .reason = ExitReason::HostReboot;

    assert_eq!(session_plan(&session), SessionPlan::Resume);
}

#[test]
fn inconsistent_non_natural_exit_cannot_start_provider_recovery() {
    let mut session = completed_session(Uuid::new_v4());
    session.completed_attachment = None;
    let runtime = session.runtime.as_mut().unwrap();
    runtime.exit.as_mut().unwrap().reason = ExitReason::HostReboot;
    runtime.last_output_sequence = MISMATCHED_OUTPUT_SEQUENCE;

    assert_eq!(session_plan(&session), SessionPlan::PendingFinalization);
}

#[tokio::test]
async fn prepare_completed_session_does_not_resume_or_emit_resumed() {
    let id = Uuid::new_v4();
    let expected = completed_session(id);
    let host = FakeHost::start(vec![Reply::value(&host_status()), Reply::value(&expected)]).await;
    let (status, mut statuses) = mpsc::unbounded_channel();

    let prepared = RelayRuntime::existing(host.client())
        .prepare(id, &status)
        .await
        .unwrap();

    assert!(matches!(prepared, PreparedSession::Completed { .. }));
    assert!(statuses.try_recv().is_err());
    assert_eq!(host.methods(), [method::HOST_STATUS, method::GET_SESSION]);
}

#[tokio::test]
async fn prepare_pending_natural_exit_rejects_without_resume() {
    let id = Uuid::new_v4();
    let mut pending = completed_session(id);
    pending.completed_attachment = None;
    let host = FakeHost::start(vec![Reply::value(&host_status()), Reply::value(&pending)]).await;
    let (status, _) = mpsc::unbounded_channel();

    assert!(matches!(
        RelayRuntime::existing(host.client()).prepare(id, &status).await,
        Err(RelayError::SessionNotAttachable(actual)) if actual == id
    ));
    assert_eq!(host.methods(), [method::HOST_STATUS, method::GET_SESSION]);
}

#[test]
fn completed_attestation_cannot_override_live_status() {
    let mut session = completed_session(Uuid::new_v4());
    session.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Running;

    assert_eq!(session_plan(&session), SessionPlan::PendingFinalization);
}

#[tokio::test]
async fn user_termination_stops_automatic_reconnect_without_resuming_or_claiming_output_complete() {
    let id = Uuid::new_v4();
    let mut stopped = completed_session(id);
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
    let (status, mut statuses) = mpsc::unbounded_channel();
    assert!(matches!(
        RelayRuntime::existing(host.client()).prepare(id, &status).await,
        Err(RelayError::SessionNotAttachable(actual)) if actual == id
    ));
    assert_eq!(statuses.try_recv().unwrap(), RelayStatus::SessionEnding);
    assert!(statuses.try_recv().is_err());
    assert_eq!(host.methods(), [method::HOST_STATUS, method::GET_SESSION]);
}

#[test]
fn inconsistent_user_termination_cannot_claim_terminal_authority() {
    let mut stopped = completed_session(Uuid::new_v4());
    stopped.completed_attachment = None;
    let runtime = stopped.runtime.as_mut().unwrap();
    runtime.exit.as_mut().unwrap().reason = ExitReason::UserTerminate;
    runtime.last_output_sequence = MISMATCHED_OUTPUT_SEQUENCE;
    assert_eq!(session_plan(&stopped), SessionPlan::PendingFinalization);
}
