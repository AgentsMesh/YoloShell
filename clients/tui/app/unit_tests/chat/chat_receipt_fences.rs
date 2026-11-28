use super::{chat_support::*, interrupt_support};
use crate::{
    Mode, chat,
    network::{Job, Update},
};
use crossterm::event::{KeyCode, KeyModifiers};
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_tui_chat::SendPhase;

#[test]
fn semantic_errors_are_correlated_and_distinguish_rejection_from_uncertain_delivery() {
    for (error, phase) in [
        (
            ControlError::Client(ClientError::Remote(PublicError::Unauthorized)),
            SendPhase::Rejected(FailureCode::PermissionDenied),
        ),
        (
            ControlError::Client(ClientError::Remote(PublicError::Unsupported)),
            SendPhase::Rejected(FailureCode::Unsupported),
        ),
        (
            ControlError::Client(ClientError::Remote(PublicError::Conflict)),
            SendPhase::Rejected(FailureCode::PreconditionsChanged),
        ),
        (
            ControlError::Client(ClientError::Remote(PublicError::Internal)),
            SendPhase::Rejected(FailureCode::AdapterRejected),
        ),
        (
            ControlError::Client(ClientError::Unsupported),
            SendPhase::NotSent,
        ),
        (ControlError::QueryTaskFailed, SendPhase::Unknown),
    ] {
        let (mut state, jobs, mut incoming) = ready();
        toggle(&mut state, &jobs);
        paste(&mut state, &jobs, PROMPT);
        key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
        let request = sent(&mut incoming);
        crate::events::apply(
            &mut state,
            &jobs,
            Update::Control(ControlEvent::Finished {
                request,
                result: Err(error),
                connected: true,
            }),
        );
        assert_eq!(chat::current(&state).unwrap().composer.phase(), Some(phase));
        assert_eq!(chat::current(&state).unwrap().composer.text(), PROMPT);
    }
}

#[test]
fn every_request_correlation_field_fences_late_submission_receipts() {
    let (mut state, jobs, mut incoming) = ready();
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let request = sent(&mut incoming);
    let reply = interrupt_support::reply(&request, CommandOutcome::Delivered {});
    let scope = RequestScope::default();
    for change in 0..4 {
        let mut wrong = request.clone();
        match change {
            0 => wrong.generation = Some(uuid::Uuid::new_v4()),
            1 => wrong.destination = yolo_hub_identity::HubId::new(),
            2 => wrong.validity = None,
            _ => {
                wrong.validity = Some(
                    scope.validity_until(std::time::Instant::now() + crate::constants::RPC_TIMEOUT),
                )
            }
        }
        assert!(chat::control(&mut state, &wrong, &Ok(reply.clone()), false));
        assert!(state.control_generation.is_some());
        assert_eq!(
            chat::current(&state).unwrap().composer.phase(),
            Some(SendPhase::Sending)
        );
    }
    let mut other = request.clone();
    other.command = Command::Hello {};
    assert!(!chat::control(
        &mut state,
        &other,
        &Ok(Reply::Completed {}),
        true
    ));
    let mut interrupt = request.clone();
    let Command::AgentControl {
        request: prompt,
        grant,
    } = &request.command
    else {
        panic!()
    };
    let mut fields = prompt.fields().clone();
    fields.action = yolo_hub_agent_protocol::AgentAction::Interrupt {};
    interrupt.command = Command::AgentControl {
        request: yolo_hub_agent_protocol::AgentControlRequest::new(fields).unwrap(),
        grant: *grant,
    };
    assert!(!chat::not_sent(&mut state, &interrupt));
    assert!(chat::control(
        &mut state,
        &request,
        &Ok(Reply::Completed {}),
        false
    ));
    assert!(state.control_generation.is_none());
    assert_eq!(
        chat::current(&state).unwrap().composer.phase(),
        Some(SendPhase::Unknown)
    );
    assert!(chat::not_sent(&mut state, &request));
    assert_eq!(
        chat::current(&state).unwrap().composer.phase(),
        Some(SendPhase::Unknown)
    );
}

#[test]
fn submit_requires_current_runtime_and_authority_even_when_a_draft_is_visible() {
    for scenario in 0..5 {
        let (mut state, jobs, mut incoming) = ready();
        toggle(&mut state, &jobs);
        paste(&mut state, &jobs, PROMPT);
        let address = state.focused_terminal().unwrap();
        match scenario {
            0 => state.control_generation = None,
            1 => state.principal.client_id = uuid::Uuid::new_v4(),
            2 => {
                state.mark_offline(address);
            }
            3 => {
                let epoch = state.terminals[&address]
                    .fields()
                    .runtime
                    .unwrap()
                    .target
                    .runtime_epoch;
                state.mark_ended(address, epoch);
            }
            _ => {
                for _ in 0..crate::constants::QUEUE_CAPACITY {
                    assert!(
                        jobs.try_send(Job::Refresh {
                            request_id: uuid::Uuid::new_v4()
                        })
                        .is_ok()
                    );
                }
            }
        }
        key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
        assert!(chat::current(&state).unwrap().composer.phase().is_none());
        while let Ok(job) = incoming.try_recv() {
            assert!(matches!(job, Job::Refresh { .. }));
        }
        state.mode = Mode::Terminal;
    }
}

#[test]
fn a_previous_runtime_exit_marker_does_not_disable_the_restored_terminal_agent() {
    let (mut state, jobs, mut incoming) = ready();
    let address = state.focused_terminal().unwrap();
    state.mark_ended(address, yolo_hub_identity::RuntimeEpoch::new());
    toggle(&mut state, &jobs);
    paste(&mut state, &jobs, PROMPT);
    key(&mut state, &jobs, KeyCode::Enter, KeyModifiers::NONE);
    let request = sent(&mut incoming);
    assert!(matches!(request.command, Command::AgentControl { .. }));
}
