use super::harness_creation_readiness::{CREATED_RUNTIME_NOT_READY, wait_for_created_view};
use super::runtime_record_test_support::{record_for_identity, session_view};
use super::*;
use std::{collections::VecDeque, future::ready};
use tokio::time::Instant;

const INVALID_VIEW_RETRIED: &str = "invalid view must not be retried";
const LATE_VIEW_BUDGET: Duration = Duration::from_millis(20);

fn fixture() -> (TempDir, RuntimeRecord) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = record_for_identity(&paths, Uuid::new_v4(), identity);
    (directory, record)
}

#[tokio::test]
async fn transient_unreachable_view_waits_for_the_same_authenticated_runtime() {
    let (_directory, record) = fixture();
    let running = session_view(&record);
    let mut pending = running.clone();
    pending.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Unreachable;
    let mut replies = VecDeque::from([pending, running.clone()]);
    let observed = wait_for_created_view(&record, Instant::now() + HOST_READY_TIMEOUT, || {
        ready(Ok(replies.pop_front().unwrap()))
    })
    .await
    .unwrap();
    assert_eq!(observed.manifest.terminal_id, record.terminal_id);
    let runtime = observed.runtime.unwrap();
    assert_eq!(runtime.runtime_epoch, record.runtime_epoch);
    assert_eq!(runtime.status, TerminalRuntimeStatus::Running);
    assert!(replies.is_empty());
}

#[tokio::test]
async fn created_runtime_replacement_or_terminal_state_cannot_be_retried_away() {
    let (_directory, record) = fixture();
    let mut replaced = session_view(&record);
    replaced.runtime.as_mut().unwrap().runtime_epoch = Uuid::new_v4();
    let mut exited = session_view(&record);
    exited.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Exited;
    let mut lost = session_view(&record);
    lost.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Lost;
    let mut missing = session_view(&record);
    missing.runtime = None;
    for invalid in [replaced, exited, lost, missing] {
        let mut reply = Some(invalid);
        assert!(
            wait_for_created_view(&record, Instant::now() + HOST_READY_TIMEOUT, || {
                ready(Ok(reply.take().expect(INVALID_VIEW_RETRIED)))
            })
            .await
            .is_err()
        );
        assert!(reply.is_none());
    }
}

#[tokio::test]
async fn expired_readiness_budget_never_starts_another_inspection() {
    let (_directory, record) = fixture();
    let mut called = false;
    let result = wait_for_created_view(&record, Instant::now(), || {
        called = true;
        ready(Ok(session_view(&record)))
    })
    .await;
    assert_eq!(result.unwrap_err().to_string(), CREATED_RUNTIME_NOT_READY);
    assert!(!called);
}

#[tokio::test]
async fn a_ready_view_returned_after_the_deadline_cannot_authorize_fixture_custody() {
    let (_directory, record) = fixture();
    let deadline = Instant::now() + LATE_VIEW_BUDGET;
    let mut called = false;
    let result = wait_for_created_view(&record, deadline, || {
        called = true;
        let view = session_view(&record);
        async move {
            // Return Ready from one long poll: timeout_at alone can accept this.
            std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            Ok(view)
        }
    })
    .await;
    assert!(called);
    assert_eq!(result.unwrap_err().to_string(), CREATED_RUNTIME_NOT_READY);
}
