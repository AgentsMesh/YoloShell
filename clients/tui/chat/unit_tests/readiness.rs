use super::fixture::*;
use yolo_hub_agent_protocol::*;
use yolo_tui_chat::*;

const NEXT_OBSERVATION: u64 = 1;

#[test]
fn another_prompt_needs_a_new_ready_observation_after_delivery() {
    for outcome in [CommandOutcome::Delivered {}, CommandOutcome::Confirmed {}] {
        let (mut chat, agent) = ready();
        chat.composer.insert(PROMPT).unwrap();
        let first = chat.submit(&agent).unwrap();
        chat.composer.received(receipt(&first, outcome));
        if chat.composer.text().is_empty() {
            chat.composer.insert(FOLLOW_UP).unwrap();
        }
        let retained = chat.composer.text().to_owned();
        assert_eq!(
            chat.input_readiness(&agent),
            InputReadiness::AwaitingObservation
        );
        assert_eq!(chat.submit(&agent), Err(DraftError::ObservationPending));
        assert_eq!(chat.composer.text(), retained);
        let mut fields = agent.fields().clone();
        fields.cursor = ObservationCursor::new(
            fields.cursor.target(),
            fields.cursor.stream(),
            fields.cursor.sequence() + NEXT_OBSERVATION,
        )
        .unwrap();
        fields.capabilities.submit_prompt = Capability::Unavailable {
            reason: UnavailabilityReason::InputUnavailable,
        };
        let unavailable = AgentSnapshot::new(fields.clone()).unwrap();
        assert_eq!(
            chat.input_readiness(&unavailable),
            InputReadiness::Unavailable
        );
        assert_eq!(chat.submit(&unavailable), Err(DraftError::Unavailable));
        fields.capabilities.submit_prompt = Capability::Available {};
        let current = AgentSnapshot::new(fields).unwrap();
        assert_eq!(chat.input_readiness(&current), InputReadiness::Ready);
        let next = chat.submit(&current).unwrap();
        assert_ne!(next.fields().request_id, first.fields().request_id);
        assert_eq!(next.fields().expected_cursor, current.cursor());
        assert_eq!(chat.composer.text(), retained);
    }
}

#[test]
fn an_unknown_delivery_is_still_locked_even_after_a_new_ready_snapshot() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let request = chat.submit(&agent).unwrap();
    chat.composer
        .received(receipt(&request, CommandOutcome::Unknown {}));
    let mut fields = agent.fields().clone();
    fields.cursor = ObservationCursor::new(
        fields.cursor.target(),
        fields.cursor.stream(),
        fields.cursor.sequence() + NEXT_OBSERVATION,
    )
    .unwrap();
    let current = AgentSnapshot::new(fields).unwrap();
    assert_eq!(chat.input_readiness(&current), InputReadiness::Unavailable);
    assert_eq!(chat.submit(&current), Err(DraftError::Locked));
    assert_eq!(chat.composer.text(), PROMPT);
}
