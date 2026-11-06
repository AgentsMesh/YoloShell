use super::fixture::*;
use yolo_hub_agent_protocol::CommandOutcome;
use yolo_tui_chat::*;

#[test]
fn a_local_disconnect_can_be_resolved_only_by_the_exact_not_dispatched_result() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let first = chat.submit(&agent).unwrap();
    chat.composer.disconnected();
    assert_eq!(chat.composer.phase(), Some(SendPhase::Unknown));
    assert!(!chat.composer.not_sent(uuid::Uuid::new_v4()));
    assert_eq!(chat.submit(&agent), Err(DraftError::Locked));
    assert!(chat.composer.not_sent(first.fields().request_id));
    assert_eq!(chat.composer.phase(), Some(SendPhase::NotSent));
    assert_eq!(chat.composer.text(), PROMPT);
    assert_eq!(chat.input_readiness(&agent), InputReadiness::Ready);
    let manual = chat.submit(&agent).unwrap();
    assert_ne!(manual.fields().request_id, first.fields().request_id);
    assert_eq!(
        manual.fields().expected_cursor,
        first.fields().expected_cursor
    );
    assert!(!chat.composer.not_sent(first.fields().request_id));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Sending));
}

#[test]
fn accepted_or_unknown_provider_results_cannot_be_overwritten_by_a_late_not_sent_signal() {
    for outcome in [CommandOutcome::Accepted {}, CommandOutcome::Unknown {}] {
        let (mut chat, agent) = ready();
        chat.composer.insert(PROMPT).unwrap();
        let request = chat.submit(&agent).unwrap();
        chat.composer.received(receipt(&request, outcome));
        chat.composer.disconnected();
        assert_eq!(chat.composer.phase(), Some(SendPhase::Unknown));
        assert!(!chat.composer.not_sent(request.fields().request_id));
        assert_eq!(chat.submit(&agent), Err(DraftError::Locked));
        assert_eq!(chat.composer.text(), PROMPT);
    }
}
