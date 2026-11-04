use super::{
    constants::*,
    network::{Operation, Reading},
};
use crate::State;
use std::time::Instant;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_agent_protocol::{AgentAction, CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_tui_chat::{QueryError, SendPhase};

pub(crate) fn completed(
    state: &mut State,
    operation: Operation,
    result: Result<Reading, QueryError>,
) {
    super::synchronize(state);
    if state.control_generation != Some(operation.generation)
        || !state.client_capability_enabled(ClientCapability::Conversations)
    {
        return;
    }
    let Some(entry) = state.chats.entries.get_mut(&operation.key) else {
        return;
    };
    if entry.view.retired() {
        return;
    }
    match result {
        Ok(Reading::Resolved(descriptor)) => {
            entry.view.resolved(&operation.query, descriptor);
        }
        Ok(Reading::Page(page)) => {
            entry.view.loaded(&operation.query, page);
        }
        Err(error) => {
            entry.view.fail(&operation.query, error);
        }
    }
    entry.next = Instant::now() + POLL;
}

pub(crate) fn control(
    state: &mut State,
    request: &ControlRequest,
    result: &Result<Reply, ControlError>,
    connected: bool,
) -> bool {
    let Command::AgentControl {
        request: agent_request,
        ..
    } = &request.command
    else {
        return false;
    };
    if !matches!(
        agent_request.fields().action,
        AgentAction::SubmitPrompt { .. }
    ) {
        return false;
    }
    let Some(entry) = state.chats.entries.values_mut().find(|entry| {
        entry
            .submission
            .as_ref()
            .is_some_and(|pending| matches_request(&pending.request, request))
    }) else {
        return true;
    };
    let previous = entry.view.composer.phase();
    match result {
        Ok(Reply::AgentControl { receipt }) => {
            entry.view.composer.received(*receipt);
        }
        Err(error) if known_not_sent(error) => {
            entry
                .view
                .composer
                .not_sent(agent_request.fields().request_id);
        }
        other => {
            let outcome = match other {
                Err(ControlError::Client(error)) if error.outcome_is_unknown() => {
                    CommandOutcome::Unknown {}
                }
                Err(ControlError::Client(ClientError::Remote(error))) => CommandOutcome::Failed {
                    code: match error {
                        PublicError::Unauthorized => FailureCode::PermissionDenied,
                        PublicError::Unsupported => FailureCode::Unsupported,
                        PublicError::Conflict | PublicError::TargetMismatch => {
                            FailureCode::PreconditionsChanged
                        }
                        _ => FailureCode::AdapterRejected,
                    },
                },
                _ => CommandOutcome::Unknown {},
            };
            if let Ok(mut receipt) = CommandReceipt::accepted(
                agent_request.fields().request_id,
                agent_request.fields().target,
            ) {
                if receipt.advance(outcome).is_ok() {
                    entry.view.composer.received(receipt);
                }
            }
        }
    }
    if request.generation == state.control_generation
        && previous != entry.view.composer.phase()
        && entry.view.composer.phase()
            == Some(SendPhase::Rejected(FailureCode::PreconditionsChanged))
    {
        // A rejected precondition may concern the writer, without changing the agent cursor.
        state.chats.catalog_invalidated = true;
    }
    if request.generation == state.control_generation && !connected {
        state.disconnected();
    }
    true
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest) -> bool {
    control(state, request, &Err(ControlError::NotConnected), true)
}

fn known_not_sent(error: &ControlError) -> bool {
    matches!(
        error,
        ControlError::NotConnected
            | ControlError::StaleGeneration
            | ControlError::Invalidated
            | ControlError::OperationLimit
            | ControlError::QueryLimit
            | ControlError::Client(
                ClientError::Unsupported | ClientError::Validation(_) | ClientError::WrongLocalUser
            )
    )
}
fn matches_request(expected: &ControlRequest, actual: &ControlRequest) -> bool {
    expected.generation == actual.generation
        && expected.destination == actual.destination
        && expected.command == actual.command
        && expected
            .validity
            .as_ref()
            .zip(actual.validity.as_ref())
            .is_some_and(|(left, right)| {
                left.same_scope(right) && left.deadline() == right.deadline()
            })
}
