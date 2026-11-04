use super::{chat_support::*, interrupt_support};
use crate::network::Update;
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_hub_agent_protocol::{AgentAction, CommandOutcome};
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::Command;

#[test]
fn enter_uses_exact_agent_control_and_delivery_does_not_claim_processing() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let queued = sent(&mut incoming);
    let Command::AgentControl { request, grant } = &queued.command else {
        panic!()
    };
    assert_eq!(
        request.fields().target,
        crate::chat::current(&state).unwrap().binding().agent
    );
    assert_eq!(grant.fields().principal, state.principal);
    assert!(
        matches!(&request.fields().action, AgentAction::SubmitPrompt { prompt } if prompt.text() == PROMPT)
    );
    assert!(queued.validity.as_ref().unwrap().is_current());
    assert!(
        crate::interrupt_events::finished(
            &mut state,
            &queued,
            &Ok(interrupt_support::reply(
                &queued,
                CommandOutcome::Delivered {}
            ))
        )
        .is_none()
    );
    let reply = interrupt_support::reply(&queued, CommandOutcome::Delivered {});
    interrupt_support::finish(&mut state, &jobs, queued.clone(), reply);
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.phase(),
        Some(yolo_tui_chat::SendPhase::Delivered)
    );
    paste(&mut state, &jobs, NEXT_PROMPT);
    let reply = interrupt_support::reply(&queued, CommandOutcome::Delivered {});
    interrupt_support::finish(&mut state, &jobs, queued, reply);
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.text(),
        NEXT_PROMPT
    );
}

#[test]
fn uncertainty_is_not_replayed_and_explicit_discard_only_changes_the_local_draft() {
    for response in [
        Ok(CommandOutcome::Unknown {}),
        Err(ControlError::Client(yolo_hub_client::ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
        ))),
    ] {
        let (mut state, jobs, mut incoming) = ready();
        toggle(&mut state, &jobs);
        paste(&mut state, &jobs, PROMPT);
        key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
        let queued = sent(&mut incoming);
        let result = response.map(|outcome| interrupt_support::reply(&queued, outcome));
        crate::events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Finished {
                request: queued,
                result,
                connected: true,
            }),
        );
        assert_eq!(
            crate::chat::current(&state).unwrap().composer.phase(),
            Some(yolo_tui_chat::SendPhase::Unknown)
        );
        key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
        assert!(incoming.try_recv().is_err());
        key(&mut state, &jobs, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(
            crate::chat::current(&state)
                .unwrap()
                .composer
                .text()
                .is_empty()
        );
        assert!(incoming.try_recv().is_err());
    }
}

#[test]
fn missing_or_foreign_writer_and_local_queue_failure_keep_the_draft() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    let address = state.focused_terminal().unwrap();
    let writer = state.writers.remove(&address).unwrap();
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    assert!(incoming.try_recv().is_err());
    state.writers.insert(address, writer);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let queued = sent(&mut incoming);
    crate::channel_events::not_sent(&mut state, queued);
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.phase(),
        Some(yolo_tui_chat::SendPhase::NotSent)
    );
    assert_eq!(
        crate::chat::current(&state).unwrap().composer.text(),
        PROMPT
    );
}
