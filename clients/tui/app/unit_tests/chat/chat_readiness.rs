use super::{chat_support::*, drawing_support, interrupt_support};
use crate::{chat, network::Job};
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_hub_agent_protocol::{Capability, CommandOutcome, ObservationCursor};
use yolo_tui_chat::InputReadiness;

const NEXT_OBSERVATION: u64 = 1;
const REFRESHING: &str = "Waiting for current agent state";
const READY: &str = "Append & send";

#[test]
fn stale_chat_footer_status_resolves_when_its_condition_does() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let request = sent(&mut incoming);
    // A repeated Enter while the operation is in flight reports a local lock.
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(state.status, yolo_tui_chat::DRAFT_LOCKED);
    let reply = interrupt_support::reply(&request, CommandOutcome::Delivered {});
    interrupt_support::finish(&mut state, &jobs, request, reply);
    chat::pump(&mut state, &jobs);
    assert!(state.status.is_empty());

    // The unavailable message also clears once a current observation permits
    // input; it does not depend on another send attempt.
    interrupt_support::mutate_agent(&mut state, |fields| {
        fields.cursor = ObservationCursor::new(
            fields.cursor.target(),
            fields.cursor.stream(),
            fields.cursor.sequence() + NEXT_OBSERVATION,
        )
        .unwrap();
        fields.capabilities.submit_prompt = Capability::Unsupported {};
    });
    paste(&mut state, &jobs, NEXT_PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(state.status, yolo_tui_chat::DRAFT_UNAVAILABLE);
    interrupt_support::mutate_agent(&mut state, |fields| {
        fields.capabilities.submit_prompt = Capability::Available {};
    });
    chat::pump(&mut state, &jobs);
    assert!(state.status.is_empty());
}

#[test]
fn delivered_prompt_refreshes_readiness_and_keeps_a_second_draft_until_explicit_submit() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let first = sent(&mut incoming);
    let yolo_hub_rpc_protocol::Command::AgentControl { request, .. } = &first.command else {
        panic!()
    };
    let first_id = request.fields().request_id;
    let first_cursor = request.fields().expected_cursor;
    let reply = interrupt_support::reply(&first, CommandOutcome::Delivered {});
    interrupt_support::finish(&mut state, &jobs, first, reply);
    paste(&mut state, &jobs, NEXT_PROMPT);
    assert_eq!(
        chat::input_readiness(&state),
        InputReadiness::AwaitingObservation
    );
    let text = drawing_support::render(&state, WIDTH, HEIGHT).0;
    assert!(text.contains(REFRESHING));
    assert!(!text.contains(READY));
    chat::pump(&mut state, &jobs);
    assert!(matches!(incoming.try_recv().unwrap(), Job::Refresh { .. }));
    while let Ok(job) = incoming.try_recv() {
        assert!(matches!(job, Job::Chat(_)));
    }
    assert_eq!(chat::current(&state).unwrap().composer.text(), NEXT_PROMPT);
    chat::pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    interrupt_support::mutate_agent(&mut state, |fields| {
        fields.cursor = ObservationCursor::new(
            fields.cursor.target(),
            fields.cursor.stream(),
            fields.cursor.sequence() + NEXT_OBSERVATION,
        )
        .unwrap();
    });
    assert_eq!(chat::input_readiness(&state), InputReadiness::Ready);
    assert!(
        drawing_support::render(&state, WIDTH, HEIGHT)
            .0
            .contains(READY)
    );
    assert!(incoming.try_recv().is_err());
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let second = sent(&mut incoming);
    let yolo_hub_rpc_protocol::Command::AgentControl { request, .. } = &second.command else {
        panic!()
    };
    assert_ne!(request.fields().request_id, first_id);
    assert_ne!(request.fields().expected_cursor, first_cursor);
    assert_eq!(
        request.fields().expected_cursor,
        chat::agent(&state).unwrap().cursor()
    );
}
