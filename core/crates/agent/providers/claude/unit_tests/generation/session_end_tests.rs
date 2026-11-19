use super::super::termination::SessionEndBoundary;
use super::super::{
    GenerationApplyDisposition, SessionEndReason, SessionEndTermination, SessionStartSource,
};
use super::support::*;
use crate::agent::{AgentBindingStore, AgentRecoveryAvailability, AgentState};

#[test]
fn session_end_commits_recovering_binding_with_exact_process_identity() {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let (running, _) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, SessionStartSource::Fresh),
        )
        .unwrap();
    let running_binding = running.binding(None).unwrap();
    let binding_store = AgentBindingStore::new(fixture.binding_path());
    binding_store
        .save_authoritative_in_namespace(
            &fixture.lease,
            &running_binding,
            &running,
            fixture.runtime(),
        )
        .unwrap();

    let end = end(&session_id);
    let end_sequence = running
        .last_event_sequence
        .checked_add(crate::agent::claude::generation::constants::REVISION_INCREMENT)
        .unwrap();
    let (ended, disposition, reactivated) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &running,
            end_sequence,
            &end,
            None,
        )
        .unwrap();
    assert!(!reactivated);
    let (ended, ended_binding) = fixture
        .store
        .publish_binding_state_in_namespace(
            &fixture.lease,
            &fixture.binding_path(),
            fixture.authentication(),
            &ended,
            false,
            |_| Ok(()),
        )
        .unwrap();

    assert_eq!(disposition, GenerationApplyDisposition::Applied);
    assert_eq!(ended.state, AgentState::Unknown);
    assert_eq!(ended.transcript_path, None);
    assert_eq!(ended.transcript_identity, None);
    assert_eq!(ended.binding_revision, None);
    assert_eq!(
        ended.termination_candidate,
        Some(SessionEndTermination {
            reason: SessionEndReason::Other,
            boundary: SessionEndBoundary::FinalCandidate,
        })
    );
    assert_eq!(ended_binding.state, AgentState::Unknown);
    assert_eq!(
        ended_binding.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(ended_binding.process_pid, Some(PROCESS_PID));
    assert_eq!(ended_binding.process_start_time, Some(PROCESS_START));
    assert!(ended.matches_unknown_binding(&ended_binding));
    assert!(
        !fixture
            .store
            .binding_is_published_in_namespace(&fixture.lease, fixture.authentication(), &ended,)
            .unwrap()
    );
    assert_eq!(
        binding_store.read_raw_in_namespace(&fixture.lease).unwrap(),
        Some(ended_binding)
    );

    let (replayed, replay) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &end)
        .unwrap();
    assert_eq!(replay, GenerationApplyDisposition::Replay);
    assert_eq!(replayed, ended);
    let (unchanged, duplicate) = fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &super::support::end(&session_id),
        )
        .unwrap();
    assert_eq!(duplicate, GenerationApplyDisposition::Invalidated);
    assert_eq!(unchanged, ended);
}
