use super::super::super::{ClaudeGenerationAuthority, SessionEndReason, SessionStartSource};
use super::super::support::*;

pub(super) fn ended_fixture(reason: SessionEndReason) -> (Fixture, ClaudeGenerationAuthority) {
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
    fixture
        .lease
        .write_json(&fixture.binding_path(), &running_binding)
        .unwrap();
    let end = end_with_reason(&session_id, reason);
    let sequence = running.last_event_sequence + super::super::super::constants::REVISION_INCREMENT;
    let (ended, _, _) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            fixture.authentication(),
            &running,
            sequence,
            &end,
            None,
        )
        .unwrap();
    fixture
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
    (fixture, ended)
}
