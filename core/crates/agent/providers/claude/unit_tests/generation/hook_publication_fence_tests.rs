use super::super::GenerationApplyDisposition;
use super::durable_state_validation_tests::durable_snapshot;
use super::hook_publication_fence_test_support::*;
use super::support::end;
use crate::agent::{AgentRecoveryAvailability, AgentState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn session_end_finishes_recovering_binding_with_live_hook_proof() {
    let (fixture, running) = published_fixture();
    let transcript = running.transcript_path.clone().unwrap();
    let transcript_identity = running.transcript_identity.unwrap();
    let end = end(&running.provider_session_id);
    let (ended, disposition, _) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &running,
            next_sequence(&running),
            &end,
            None,
        )
        .unwrap();
    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    let admission = end_admission(
        &ended,
        transcript,
        transcript_identity,
        Arc::new(AtomicBool::new(true)),
    );

    let (committed, binding) = fixture
        .store
        .publish_hook_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &ended,
            false,
            &admission,
            |_| Ok(()),
        )
        .unwrap();

    assert_eq!(committed.state, AgentState::Unknown);
    assert_eq!(committed.binding_revision, None);
    assert_eq!(binding.state, AgentState::Unknown);
    assert_eq!(
        binding.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(binding.process_pid, Some(running.process.pid));
    assert_eq!(binding.process_start_time, Some(running.process.start_time));
}

#[test]
fn pending_and_forced_publications_revalidate_the_live_fence() {
    assert_rejected_at_pending_commit();
    assert_rejected_at_forced_refresh();
    assert_rejected_at_published_return();
}

fn assert_rejected_at_pending_commit() {
    let (fixture, running) = published_fixture();
    let end = end(&running.provider_session_id);
    let transcript = running.transcript_path.clone().unwrap();
    let transcript_identity = running.transcript_identity.unwrap();
    let (ended, _, _) = fixture
        .store
        .transition_foreground_durable_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &running,
            next_sequence(&running),
            &end,
            None,
        )
        .unwrap();
    let gate = Arc::new(AtomicBool::new(true));
    let admission = end_admission(&ended, transcript, transcript_identity, Arc::clone(&gate));
    gate.store(false, Ordering::Release);
    let before = durable_snapshot(&fixture);
    assert!(publish(&fixture, &ended, false, &admission).is_err());
    assert_eq!(durable_snapshot(&fixture), before);
}

fn assert_rejected_at_forced_refresh() {
    let (fixture, running) = published_fixture();
    let gate = Arc::new(AtomicBool::new(true));
    let admission = admission(&running, Arc::clone(&gate));
    gate.store(false, Ordering::Release);
    let before = durable_snapshot(&fixture);
    assert!(publish(&fixture, &running, true, &admission).is_err());
    assert_eq!(durable_snapshot(&fixture), before);
}

fn assert_rejected_at_published_return() {
    let (fixture, running) = published_fixture();
    let gate = Arc::new(AtomicBool::new(true));
    let admission = admission(&running, Arc::clone(&gate));
    gate.store(false, Ordering::Release);
    let before = durable_snapshot(&fixture);
    assert!(publish(&fixture, &running, false, &admission).is_err());
    assert_eq!(durable_snapshot(&fixture), before);
}
