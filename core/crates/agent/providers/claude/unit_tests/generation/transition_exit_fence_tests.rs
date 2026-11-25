use super::super::{GenerationApplyDisposition, GenerationEvent, SessionStartSource};
use super::support::*;
use crate::agent::AgentState;

const LATE_CWD_COMPONENT: &str = "late";

#[test]
fn same_session_late_events_after_session_end_are_fenced_without_mutation() {
    let fixture = Fixture::new();
    let session_id = fixture.begin.provider_session_id.clone();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    fixture
        .store
        .apply_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &fixture.start(&session_id, SessionStartSource::Fresh),
        )
        .unwrap();
    let (ended, _) = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &end(&session_id))
        .unwrap();
    let events = [
        GenerationEvent::StateChanged {
            event_id: uuid::Uuid::new_v4(),
            provider_session_id: session_id.clone(),
            state: AgentState::WaitingForInput,
        },
        GenerationEvent::CwdChanged {
            event_id: uuid::Uuid::new_v4(),
            provider_session_id: session_id.clone(),
            cwd: fixture.begin.cwd.join(LATE_CWD_COMPONENT),
            transcript_path: fixture.transcript(&session_id),
        },
        end(&session_id),
    ];
    for event in events {
        let (unchanged, disposition) = fixture
            .store
            .apply_in_namespace(&fixture.lease, fixture.authentication(), &event)
            .unwrap();
        assert_eq!(disposition, GenerationApplyDisposition::Invalidated);
        assert_eq!(unchanged, ended);
    }
}
