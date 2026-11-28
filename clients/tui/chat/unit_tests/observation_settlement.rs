use super::fixture::*;
use yolo_hub_agent_protocol::*;
use yolo_tui_chat::*;

fn next_cursor(agent: &AgentSnapshot) -> ObservationCursor {
    ObservationCursor::new(
        agent.cursor().target(),
        agent.cursor().stream(),
        agent.cursor().sequence() + 1,
    )
    .unwrap()
}

#[test]
fn newer_observation_settles_completed_receipts_but_not_uncertain_or_undispatched_ones() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let request = chat.submit(&agent).unwrap();
    // A receipt is still pending; a changed observation cannot settle it.
    assert!(!chat.composer.observe(next_cursor(&agent)));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Sending));

    assert!(
        chat.composer
            .received(receipt(&request, CommandOutcome::Delivered {}))
    );
    assert!(!chat.composer.observe(agent.cursor()));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Delivered));
    assert!(chat.composer.observe(next_cursor(&agent)));
    assert_eq!(chat.composer.phase(), None);
    assert!(chat.composer.text().is_empty());

    // Accepted is a final receipt for the request; without this settlement it
    // would lock the composer forever when no later receipt arrives.
    chat.composer.insert(PROMPT).unwrap();
    let accepted = chat.submit(&agent).unwrap();
    assert!(
        chat.composer
            .received(receipt(&accepted, CommandOutcome::Accepted {}))
    );
    assert!(chat.composer.locked());
    assert!(chat.composer.observe(next_cursor(&agent)));
    assert!(!chat.composer.locked());

    chat.composer.insert(PROMPT).unwrap();
    let unknown = chat.submit(&agent).unwrap();
    assert!(
        chat.composer
            .received(receipt(&unknown, CommandOutcome::Unknown {}))
    );
    assert!(!chat.composer.observe(next_cursor(&agent)));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Unknown));
    assert!(chat.composer.discard_uncertain());

    chat.composer.insert(PROMPT).unwrap();
    let not_sent = chat.submit(&agent).unwrap();
    assert!(chat.composer.not_sent(not_sent.fields().request_id));
    assert!(!chat.composer.observe(next_cursor(&agent)));
    assert_eq!(chat.composer.phase(), Some(SendPhase::NotSent));

    chat.composer.clear().unwrap();
    chat.composer.insert(PROMPT).unwrap();
    let rejected = chat.submit(&agent).unwrap();
    assert!(chat.composer.received(CommandReceipt::rejected(
        &rejected,
        FailureCode::PreconditionsChanged
    )));
    assert!(!chat.composer.observe(next_cursor(&agent)));
    assert_eq!(
        chat.composer.phase(),
        Some(SendPhase::Rejected(FailureCode::PreconditionsChanged))
    );
}
