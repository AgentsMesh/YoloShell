use crate::{State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{CreateShellRequest, CreateShellResult};

pub(crate) fn created(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    request: CreateShellRequest,
    creation: CreateShellResult,
) {
    let generation = state.control_generation;
    created_from_generation(state, jobs, request, creation, generation);
}

pub(crate) fn created_from_generation(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    request: CreateShellRequest,
    creation: CreateShellResult,
    generation: Option<uuid::Uuid>,
) {
    let id = request.fields().request_id;
    let matches = state.creations.pending.get(&id).is_some_and(|pending| {
        pending.request == request
            && pending.origin == creation.address.hub_id
            && pending.generation == generation
    });
    if !matches {
        return;
    }
    let Some(pending) = state.creations.pending.get_mut(&id) else {
        return;
    };
    if pending.confirmed.is_some_and(|known| known != creation) {
        return;
    }
    pending.confirmed = Some(creation);
    let now = Instant::now();
    crate::creation_confirmation::flush(state, jobs, now);
    crate::finalization_actions::flush(state, jobs, now);
}

pub(crate) fn retire_command(state: &mut State, command: &Command) {
    if let Command::Create { request } = command {
        state.creations.retire(request);
    }
}
