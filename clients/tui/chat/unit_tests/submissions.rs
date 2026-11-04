use super::fixture::*;
use yolo_hub_agent_protocol::*;
use yolo_tui_chat::*;

#[test]
fn input_intent_requires_matching_execution_live_capability_and_a_nonempty_draft() {
    let (mut chat, agent) = ready();
    assert_eq!(chat.submit(&agent), Err(DraftError::Empty));
    chat.composer.insert(" \n\t ").unwrap();
    assert_eq!(chat.submit(&agent), Err(DraftError::Empty));
    chat.composer.clear().unwrap();
    chat.composer.insert(PROMPT).unwrap();
    chat.set_mode(Mode::Terminal);
    assert_eq!(chat.submit(&agent), Err(DraftError::WrongTarget));
    chat.set_mode(Mode::Chat);
    assert_eq!(
        chat.submit(&super::fixture::agent()),
        Err(DraftError::WrongTarget)
    );
    let mut fields = agent.fields().clone();
    fields.capabilities.submit_prompt = Capability::Unsupported {};
    assert_eq!(
        chat.submit(&AgentSnapshot::new(fields).unwrap()),
        Err(DraftError::Unavailable)
    );
    let request = chat.submit(&agent).unwrap();
    assert_eq!(request.fields().target, chat.binding().agent);
    assert_eq!(request.fields().expected_cursor, agent.cursor());
    let AgentAction::SubmitPrompt { prompt } = &request.fields().action else {
        panic!()
    };
    assert_eq!(prompt.text(), PROMPT);
    assert_eq!(chat.composer.request(), Some(&request));
    assert_eq!(chat.submit(&agent), Err(DraftError::Locked));
}

#[test]
fn receipt_stages_do_not_claim_processing_and_duplicate_delivery_cannot_clear_a_new_draft() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let request = chat.submit(&agent).unwrap();
    assert_eq!(chat.composer.phase(), Some(SendPhase::Sending));
    let accepted = receipt(&request, CommandOutcome::Accepted {});
    assert!(chat.composer.received(accepted));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Accepted));
    assert_eq!(chat.composer.text(), PROMPT);
    let delivered = receipt(&request, CommandOutcome::Delivered {});
    assert!(chat.composer.received(delivered));
    assert_eq!(chat.composer.phase(), Some(SendPhase::Delivered));
    assert!(SendPhase::Delivered.label().contains("not yet confirmed"));
    chat.composer.insert(FOLLOW_UP).unwrap();
    assert!(chat.composer.received(delivered));
    assert_eq!(chat.composer.text(), FOLLOW_UP);
    assert!(!chat.composer.received(accepted));
    assert!(
        !chat
            .composer
            .received(receipt(&request, CommandOutcome::Unknown {}))
    );
    assert!(
        chat.composer
            .received(receipt(&request, CommandOutcome::Confirmed {}))
    );
    assert_eq!(chat.composer.text(), FOLLOW_UP);
    assert_eq!(chat.composer.phase(), Some(SendPhase::Confirmed));
    assert!(!chat.composer.received(delivered));
    chat.composer.disconnected();
    assert_eq!(chat.composer.phase(), Some(SendPhase::Confirmed));
}

#[test]
fn unknown_result_never_replays_and_only_matching_later_evidence_resolves_it() {
    let (mut chat, agent) = ready();
    chat.composer.insert(PROMPT).unwrap();
    let request = chat.submit(&agent).unwrap();
    chat.composer.disconnected();
    assert_eq!(chat.composer.phase(), Some(SendPhase::Unknown));
    assert_eq!(chat.submit(&agent), Err(DraftError::Locked));
    let unknown = receipt(&request, CommandOutcome::Unknown {});
    assert!(chat.composer.received(unknown));
    assert!(!chat.composer.not_sent(request.fields().request_id));
    assert!(
        !chat
            .composer
            .received(receipt(&request, CommandOutcome::Accepted {}))
    );
    let wrong = CommandReceipt::accepted(uuid::Uuid::new_v4(), request.fields().target).unwrap();
    assert!(!chat.composer.received(wrong));
    let wrong = CommandReceipt::accepted(
        request.fields().request_id,
        super::fixture::agent().cursor().target(),
    )
    .unwrap();
    assert!(!chat.composer.received(wrong));
    assert!(
        chat.composer
            .received(receipt(&request, CommandOutcome::Delivered {}))
    );
    assert!(chat.composer.text().is_empty());
}

#[test]
fn known_queue_rejection_keeps_draft_and_allows_only_an_explicit_fresh_submission() {
    let (mut chat, agent) = ready();
    assert!(!chat.composer.not_sent(uuid::Uuid::new_v4()));
    chat.composer.disconnected();
    let unrelated =
        CommandReceipt::accepted(uuid::Uuid::new_v4(), agent.cursor().target()).unwrap();
    assert!(!chat.composer.received(unrelated));
    chat.composer.insert(PROMPT).unwrap();
    let request = chat.submit(&agent).unwrap();
    assert!(!chat.composer.not_sent(uuid::Uuid::new_v4()));
    assert!(chat.composer.not_sent(request.fields().request_id));
    assert_eq!(chat.composer.phase(), Some(SendPhase::NotSent));
    assert_eq!(chat.composer.text(), PROMPT);
    assert!(
        !chat
            .composer
            .received(receipt(&request, CommandOutcome::Delivered {}))
    );
    let next = chat.submit(&agent).unwrap();
    assert_ne!(next.fields().request_id, request.fields().request_id);
    let rejected = CommandReceipt::rejected(&next, FailureCode::PreconditionsChanged);
    assert!(chat.composer.received(rejected));
    assert_eq!(
        chat.composer.phase(),
        Some(SendPhase::Rejected(FailureCode::PreconditionsChanged))
    );
    assert_eq!(chat.composer.text(), PROMPT);
    assert!(
        !chat
            .composer
            .received(receipt(&request, CommandOutcome::Delivered {}))
    );
    for phase in [
        SendPhase::Sending,
        SendPhase::Accepted,
        SendPhase::Delivered,
        SendPhase::Confirmed,
        SendPhase::Unknown,
        SendPhase::NotSent,
        SendPhase::Rejected(FailureCode::Unsupported),
    ] {
        assert!(!phase.label().is_empty());
    }
}
