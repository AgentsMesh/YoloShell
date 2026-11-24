use crate::{State, creation_constants::*, creation_state::PendingCreation, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::CreateShellRequest;
use yolo_tui_creation_journal::CreationIntent;

pub(crate) fn enqueue(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    origin: HubId,
    request: CreateShellRequest,
) -> bool {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalLifecycle,
        crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE,
    ) {
        return false;
    }
    if let Some(error) = state.creations.persistence_error {
        state.status = error.into();
        return false;
    }
    let id = request.fields().request_id;
    if state.creations.len() >= MAX_PENDING_CREATIONS
        || state.creations.pending.contains_key(&id)
        || state.creations.recovery.contains_request(&request)
        || state
            .creations
            .finalizing
            .contains_key(&request.fields().creation_token)
        || state.creations.pending.values().any(|pending| {
            pending.request.fields().creation_token == request.fields().creation_token
        })
        || state
            .creations
            .finalizing
            .values()
            .any(|entry| entry.record.request.fields().request_id == id)
    {
        state.status = CREATION_BUSY.into();
        return false;
    }
    let Some(generation) = state
        .control_generation
        .filter(|generation| !generation.is_nil())
    else {
        state.status = crate::constants::DISCONNECTED_LABEL.into();
        return false;
    };
    // Reserve capacity before durable preparation. No Create can leave this
    // process until the journal commit succeeds and this permit is consumed.
    let Ok(permit) = jobs.try_reserve() else {
        state.status = crate::constants::QUEUE_FULL_LABEL.into();
        return false;
    };
    if !crate::creation_recovery_persistence::prepare(
        state,
        CreationIntent {
            origin,
            request: request.clone(),
        },
        Instant::now(),
    ) {
        return false;
    }
    let command = Command::Create {
        request: request.clone(),
    };
    let pending = PendingCreation {
        request: request.clone(),
        origin,
        tab: state.tabs.active().id,
        pane: state.tab_layout().focused(),
        expected: state.focused_terminal(),
        binding_active: true,
        confirmed: None,
        confirmation_retry_at: None,
        generation: Some(generation),
        deadline: Instant::now() + command.operation_timeout(crate::constants::RPC_TIMEOUT),
        presentation: crate::creation_presentation::CreationPresentation::capture(state),
        ui_epoch: state.ui_epoch,
        reservation: None,
    };
    state.creations.retire_binding(pending.tab, pending.pane);
    state.creations.pending.insert(id, pending);
    permit.send(Job::Call {
        generation: Some(generation),
        origin,
        command,
    });
    true
}
