use crate::{
    State,
    constants::*,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::{Command, Reply};

pub fn enqueue(state: &mut State, jobs: &mpsc::Sender<Job>, mut job: Job) -> bool {
    if matches!(
        &job,
        Job::Call {
            command: Command::Lease { .. }
                | Command::Renew { .. }
                | Command::Release { .. }
                | Command::Input { .. }
                | Command::Resize { .. }
                | Command::ObserveLease { .. },
            ..
        }
    ) && !crate::client_capabilities::require(
        state,
        yolo_builtin_client_contract::ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return false;
    }
    if matches!(
        &job,
        Job::Call {
            command: Command::Terminal { .. },
            ..
        }
    ) && !crate::client_capabilities::require(
        state,
        yolo_builtin_client_contract::ClientCapability::TerminalLifecycle,
        crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE,
    ) {
        return false;
    }
    if let Job::Call { generation, .. } = &mut job {
        *generation = state.control_generation;
    }
    if jobs.try_send(job).is_err() {
        state.status = QUEUE_FULL_LABEL.into();
        false
    } else {
        true
    }
}

pub fn refresh(state: &mut State, jobs: &mpsc::Sender<Job>) {
    crate::catalog_events::refresh(state, jobs);
}

pub fn apply(state: &mut State, jobs: &mpsc::Sender<Job>, update: Update) {
    state.sync_query_authority();
    apply_inner(state, jobs, update);
    state.sync_query_authority();
}

fn apply_inner(state: &mut State, jobs: &mpsc::Sender<Job>, update: Update) {
    match update {
        Update::Chat { operation, result } => crate::chat::completed(state, operation, result),
        Update::Layout { operation, result } => {
            crate::layout_sync::complete(state, operation, result);
        }
        Update::Management { operation, result } => {
            if let Some(manager) = &mut state.hub_manager {
                manager.complete(operation.token, result);
            }
            if operation.command.mutates() {
                crate::catalog_events::invalidate(state, jobs);
            }
        }
        Update::ManagementNotSent { token } => {
            if let Some(manager) = &mut state.hub_manager {
                manager.not_sent(token);
            }
        }
        Update::Control(event) => crate::channel_events::apply(state, jobs, event),
        Update::NotSent { request } => crate::channel_events::not_sent(state, request),
        Update::Observation {
            target,
            generation,
            update,
        } => {
            crate::observation_events::scoped(state, jobs, target, generation, update);
        }
        Update::Catalog(event) => {
            crate::catalog_events::apply(state, event);
            crate::initial_control::catalog(state, jobs);
        }
        Update::PlatformEvents(wake) => {
            crate::catalog_events::invalidate(state, jobs);
            if wake.lost || wake.layout_changed {
                crate::layout_sync::refresh(state);
            }
        }
        Update::Reply { command, reply } => match (command, reply) {
            (Command::ObserveLease { target }, Reply::LeaseObservation { lease }) => {
                crate::lease_events::observed(state, jobs, target, lease);
            }
            (Command::Attach { request }, Reply::Attached { attachment }) => {
                if !crate::observation_events::current(state, request.target()) {
                    return;
                }
                let address = request.target().address;
                state.retire_query_scope(address);
                state.clear_offline(address);
                let matching = state
                    .surfaces
                    .get(&address)
                    .is_some_and(|surface| surface.cursor().target == request.target());
                let result = if matching {
                    state
                        .surfaces
                        .get_mut(&address)
                        .expect("surface checked above")
                        .apply(request, &attachment)
                } else {
                    crate::observation_events::replace_surface(state, address, &attachment)
                };
                if let Err(error) = result {
                    state.surface_gap(address, Some(error.to_string()));
                } else {
                    state.inputs.snapshot_pending.remove(&address);
                    crate::resize::snapshot_applied(state, address);
                }
            }
            (Command::Lease { request }, Reply::Lease { grant }) => {
                if !crate::initial_control::granted(state, jobs, &request, grant) {
                    crate::control::granted(state, jobs, &request, grant);
                }
            }
            (Command::Renew { grant: previous }, Reply::Lease { grant }) => {
                crate::lease_events::renewed(state, jobs, previous, grant);
            }
            (Command::Create { request }, Reply::Created { creation }) => {
                crate::creation_events::created(state, jobs, request, creation);
            }
            // Lifecycle replies can arrive out of order on independent channels.
            // The control-event barrier refreshes the authoritative catalogue.
            (Command::Terminal { .. }, Reply::Terminal { .. }) => {}
            (Command::Get { .. }, Reply::Terminal { terminal: session }) => state.replace_session(session),
            (Command::Resize { request }, Reply::Completed {}) => {
                crate::resize::completed(state, &request);
            }
            (Command::CreateWorkspace { .. }, Reply::Workspace { workspace }) => crate::workspace_ui::created(state, workspace),
            (Command::UpdateWorkspace { .. }, Reply::Workspace { workspace }) => crate::workspace_ui::updated(state, workspace),
            (Command::ListWorkspaces { .. }, Reply::Workspaces { list }) => crate::workspace_ui::listed(state, jobs, list),
            (_, Reply::Completed {} | Reply::Input { .. }) => {}
            _ => state.status = UNEXPECTED_REPLY_LABEL.into(),
        },
        Update::Failed {
            command,
            message,
            connected,
            transient,
        } => {
            if let Some(command) = command.as_ref() {
                crate::initial_control::failed(state, command);
            }
            if let Some(command) = command {
                crate::creation_events::retire_command(state, &command);
                match command {
                    Command::Resize { request } => {
                        crate::resize::failed(state, &request);
                    }
                    Command::Renew { grant } | Command::Release { grant } => {
                        let address = grant.fields().target.address;
                        if state.retire_matching_writer(grant) {
                            state.clear_renewal_pending(address);
                        }
                    }
                    Command::Input { request } => {
                        state.retire_matching_writer(request.fields().grant);
                    }
                    Command::CreateWorkspace { request } => crate::workspace_ui::create_failed(state, &request, &message),
                    Command::UpdateWorkspace { change } => crate::workspace_ui::update_failed(state, &change, &message),
                    Command::ListWorkspaces { .. } => crate::workspace_ui::failed(state, &message),
                    _ => {}
                }
            }
            state.status = message.clone();
            state.recoverable_status = transient.then_some(message);
            if !connected {
                state.disconnected();
            }
        }
    }
}
