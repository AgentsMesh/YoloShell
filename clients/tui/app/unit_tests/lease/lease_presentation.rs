use super::drawing_support::*;
use crate::{events, network::Update};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseMode, LeaseRequest, PublicError};

const ERROR_LABEL: &str = "operation conflicts with current state";
const UNKNOWN_OUTCOME_LABEL: &str = "Hub operation outcome is unknown";

fn request(state: &crate::State, mode: LeaseMode) -> Command {
    let address = state.focused_terminal().unwrap();
    Command::Lease {
        request: LeaseRequest {
            request_id: uuid::Uuid::new_v4(),
            target: state.terminals[&address].fields().runtime.unwrap().target,
            principal: state.principal,
            mode,
        },
    }
}

#[test]
fn denied_acquire_keeps_identity_and_one_read_only_status_without_a_generic_error_banner() {
    let (mut state, address) = state();
    let target = state.terminals[&address].fields().runtime.unwrap().target;
    state.control.observe(target);
    state.writers.clear();
    let generation = state.control_generation;
    let command = request(&state, LeaseMode::Acquire {});
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                generation,
                destination: address.hub_id,
                command,
                validity: None,
            },
            result: Err(ControlError::Client(ClientError::Remote(
                PublicError::Conflict,
            ))),
            connected: true,
        }),
    );
    let (text, _) = render(&state, WIDTH, HEIGHT);
    assert!(text.contains(&label(SOURCE_NAME, crate::constants::READ_ONLY_LABEL)));
    assert_eq!(text.matches(crate::constants::READ_ONLY_LABEL).count(), 1);
    assert!(!text.contains(ERROR_LABEL));
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            crate::draw::draw(frame, &state);
        })
        .unwrap();
    let status = terminal
        .backend()
        .buffer()
        .content()
        .chunks(usize::from(WIDTH))
        .last()
        .unwrap()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(!status.contains(yolo_tui_render::WRITING_LABEL));
    assert_eq!(state.focused_terminal(), Some(address));
    assert!(state.writers.is_empty());
    assert!(incoming.try_recv().is_err());
}

#[test]
fn unexpected_failures_append_details_without_erasing_the_terminal_identity_or_access() {
    let (mut state, _) = state();
    for writer in state.writers.values() {
        state.control.observe(writer.grant.fields().target);
    }
    state.writers.clear();
    for error in [
        ControlError::Client(ClientError::Remote(PublicError::Unauthorized)),
        ControlError::Client(ClientError::OutcomeUnknownResponse(PublicError::Conflict)),
    ] {
        let command = request(&state, LeaseMode::Acquire {});
        state.status = crate::lease_events::failure_message(&command, &error);
        assert_eq!(state.status, error.to_string());
        let (text, _) = render(&state, NEXT_WIDTH, HEIGHT);
        assert!(text.contains(&label(SOURCE_NAME, crate::constants::READ_ONLY_LABEL)));
        assert!(text.contains(&error.to_string()));
        let (compact, _) = render(&state, WIDTH, HEIGHT);
        assert!(compact.contains(&label(SOURCE_NAME, crate::constants::READ_ONLY_LABEL)));
        if matches!(
            error,
            ControlError::Client(ClientError::OutcomeUnknownResponse(_))
        ) {
            assert!(compact.contains(UNKNOWN_OUTCOME_LABEL));
        }
        assert_eq!(state.status, error.to_string());
    }
    let command = request(
        &state,
        LeaseMode::TakeControl {
            observed_generation: yolo_hub_terminal_protocol::FIRST_LEASE_GENERATION,
        },
    );
    let error = ControlError::Client(ClientError::Remote(PublicError::Conflict));
    assert_eq!(
        crate::lease_events::failure_message(&command, &error),
        error.to_string()
    );
}

#[test]
fn a_lost_input_lease_reports_control_loss_instead_of_a_generic_hub_error() {
    let (state, address) = state();
    let terminal = state.terminals[&address].clone();
    let command = Command::Renew {
        grant: super::fixtures::grant(&terminal),
    };
    let conflict = ControlError::Client(ClientError::Remote(PublicError::Conflict));
    assert_eq!(
        crate::lease_events::failure_message(&command, &conflict),
        crate::constants::CONTROL_LOST_LABEL
    );
    // An explicit takeover still keeps its diagnostic failure.
    let takeover = request(
        &state,
        LeaseMode::TakeControl {
            observed_generation: yolo_hub_terminal_protocol::FIRST_LEASE_GENERATION,
        },
    );
    assert_eq!(
        crate::lease_events::failure_message(&takeover, &conflict),
        conflict.to_string()
    );
}

#[test]
fn a_new_grant_clears_the_control_loss_label() {
    let (mut state, address) = state();
    let terminal = state.terminals[&address].clone();
    state.status = crate::constants::CONTROL_LOST_LABEL.into();
    let (jobs, _incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::lease_events::granted(&mut state, &jobs, super::fixtures::grant(&terminal));
    assert_eq!(state.status, crate::constants::CONNECTED_LABEL);
}
