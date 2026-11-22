use super::super::termination::SessionEndBoundary;
use super::super::{GenerationApplyDisposition, SessionEndReason, SessionStartSource};
use super::support::*;
use crate::agent::AgentState;

#[test]
fn recovery_boundaries_accept_only_the_matching_start_source() {
    for (reason, accepted) in [
        (SessionEndReason::Clear, SessionStartSource::Clear),
        (SessionEndReason::Resume, SessionStartSource::Resume),
    ] {
        let fixture = Fixture::new();
        let first_id = fixture.begin.provider_session_id.clone();
        start_initial(&fixture, &first_id);
        let (recovering, _) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &end_with_reason(&first_id, reason),
            )
            .unwrap();

        let next_id = session_id();
        let (restarted, disposition) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &fixture.start(&next_id, accepted),
            )
            .unwrap();

        assert_eq!(disposition, GenerationApplyDisposition::Applied);
        assert_eq!(restarted.state, AgentState::Running);
        assert_eq!(restarted.provider_session_id, next_id);
        assert_eq!(
            restarted.generation,
            recovering.generation + super::super::constants::GENERATION_INCREMENT
        );
        assert_eq!(restarted.termination_candidate, None);
    }
}

#[test]
fn recovery_boundaries_reject_every_nonmatching_start_without_mutation() {
    for (reason, rejected) in [
        (SessionEndReason::Clear, SessionStartSource::Resume),
        (SessionEndReason::Clear, SessionStartSource::Fresh),
        (SessionEndReason::Resume, SessionStartSource::Clear),
        (SessionEndReason::Resume, SessionStartSource::Fork),
        (SessionEndReason::Other, SessionStartSource::Resume),
    ] {
        let fixture = Fixture::new();
        let session_id = fixture.begin.provider_session_id.clone();
        start_initial(&fixture, &session_id);
        let (ended, _) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &end_with_reason(&session_id, reason),
            )
            .unwrap();
        let (unchanged, disposition) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &fixture.start(&session_id, rejected),
            )
            .unwrap();

        assert_eq!(disposition, GenerationApplyDisposition::Invalidated);
        assert_eq!(unchanged, ended);
    }
}

#[test]
fn each_session_end_reason_records_its_typed_boundary() {
    for (reason, boundary) in [
        (SessionEndReason::Clear, SessionEndBoundary::Recovering),
        (SessionEndReason::Resume, SessionEndBoundary::Recovering),
        (SessionEndReason::Other, SessionEndBoundary::FinalCandidate),
    ] {
        let fixture = Fixture::new();
        let session_id = fixture.begin.provider_session_id.clone();
        start_initial(&fixture, &session_id);
        let (ended, disposition) = fixture
            .store
            .apply_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &end_with_reason(&session_id, reason),
            )
            .unwrap();

        assert_eq!(disposition, GenerationApplyDisposition::Applied);
        assert_eq!(ended.state, AgentState::Unknown);
        let candidate = ended.termination_candidate.unwrap();
        assert_eq!(candidate.reason, reason);
        assert_eq!(candidate.boundary, boundary);
    }
}

fn start_initial(fixture: &Fixture, session_id: &str) {
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(session_id, SessionStartSource::Fresh),
        )
        .unwrap();
}
