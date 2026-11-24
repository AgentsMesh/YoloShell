//! Workspace filter picker glue. The picker owns list/selection state; the
//! inventory remains the sole projection of terminals and agents.

use crate::{State, events::enqueue, network::Job};
use crossterm::event::Event;
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_workspace_protocol::{
    CreateWorkspaceRequest, ListWorkspacesRequest, ListWorkspacesResult, UpdateWorkspaceRequest,
    WorkspaceSummary,
};
use yolo_tui_workspace::Handle;

pub(crate) const UNSUPPORTED: &str = "Workspaces are not supported by this Hub";

pub(crate) fn open(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId) {
    if !state.workspaces_supported {
        state.status = UNSUPPORTED.into();
        return;
    }
    let current = state.inventory.workspace;
    let root = std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    if let Some(request) = state.workspaces.open(hub, current, root) {
        send_list(state, jobs, hub, request);
    }
}

/// Consumes every event while the picker is open; returns whether it owned it.
pub(crate) fn handle(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: HubId,
    event: &Event,
) -> bool {
    if !state.workspaces.is_open() {
        return false;
    }
    match state.workspaces.key(event) {
        Handle::None => {}
        Handle::Query(request) => send_list(state, jobs, hub, request),
        Handle::Filter(workspace) => {
            state.inventory.workspace = workspace;
            state.status = state.inventory.workspace_status(&state.terminals);
        }
        Handle::Create(request) => send_create(state, jobs, hub, request),
        Handle::Rename(request) => send_update(state, jobs, request),
    }
    true
}

pub(crate) fn listed(state: &mut State, jobs: &mpsc::Sender<Job>, list: ListWorkspacesResult) {
    let next = state.workspaces.page(list);
    if state.workspaces.is_open() && !state.workspaces.is_loading() {
        state
            .inventory
            .replace_workspaces(state.workspaces.workspaces().to_vec());
    }
    if let (Some(request), Some(hub)) = (next, state.workspaces.hub()) {
        send_list(state, jobs, hub, request);
    }
}

pub(crate) fn created(state: &mut State, workspace: WorkspaceSummary) {
    if state.workspaces.created(workspace) {
        state
            .inventory
            .replace_workspaces(state.workspaces.workspaces().to_vec());
    }
}

pub(crate) fn updated(state: &mut State, workspace: WorkspaceSummary) {
    let address = workspace.address;
    if state.workspaces.updated(workspace) {
        state
            .inventory
            .replace_workspaces(state.workspaces.workspaces().to_vec());
        if state.inventory.workspace == Some(address) {
            state.status = state.inventory.workspace_status(&state.terminals);
        }
    }
}

pub(crate) fn create_failed(state: &mut State, request: &CreateWorkspaceRequest, message: &str) {
    state
        .workspaces
        .create_failed(request.fields().workspace_id, message);
}

pub(crate) fn update_failed(state: &mut State, request: &UpdateWorkspaceRequest, message: &str) {
    state
        .workspaces
        .update_failed(request.fields().address, message);
}

pub(crate) fn failed(state: &mut State, message: &str) {
    state.workspaces.failed(message);
}

fn send_create(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: HubId,
    request: CreateWorkspaceRequest,
) {
    let workspace_id = request.fields().workspace_id;
    if !call(state, jobs, hub, Command::CreateWorkspace { request }) {
        state
            .workspaces
            .create_failed(workspace_id, crate::constants::QUEUE_FULL_LABEL);
    }
}

fn send_update(state: &mut State, jobs: &mpsc::Sender<Job>, request: UpdateWorkspaceRequest) {
    let address = request.fields().address;
    if !call(
        state,
        jobs,
        address.hub_id,
        Command::UpdateWorkspace { change: request },
    ) {
        state
            .workspaces
            .update_failed(address, crate::constants::QUEUE_FULL_LABEL);
    }
}

fn send_list(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: HubId,
    request: ListWorkspacesRequest,
) {
    if !call(state, jobs, hub, Command::ListWorkspaces { request }) {
        state.workspaces.failed(crate::constants::QUEUE_FULL_LABEL);
    }
}

fn call(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, command: Command) -> bool {
    enqueue(
        state,
        jobs,
        Job::Call {
            generation: None,
            origin: hub,
            command,
        },
    )
}
