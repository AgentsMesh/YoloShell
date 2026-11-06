use crate::{State, constants::DEFAULT_DIRECTORY, network::Job, startup_constants::*};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize,
};

pub(crate) fn destination(state: &State, _entry: HubId) -> HubId {
    state
        .focused_terminal()
        .map_or_else(|| state.tabs.machine(), |address| address.hub_id)
}

pub(crate) fn directory(state: &State, origin: HubId) -> String {
    if let Some(session) = state
        .focused_terminal()
        .and_then(|address| state.terminals.get(&address))
    {
        if session.fields().address.hub_id == origin {
            return session.fields().cwd.clone();
        }
    }
    if state.startup.hub == Some(origin) {
        if let Some(directory) = &state.startup.directory {
            return directory.clone();
        }
    }
    DEFAULT_DIRECTORY.into()
}

pub(crate) fn current_directory(state: &mut State, jobs: &mpsc::Sender<Job>, entry: HubId) {
    let origin = destination(state, entry);
    create(state, jobs, origin, directory(state, origin), None);
}

pub(crate) fn create(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    origin: HubId,
    cwd: String,
    name: Option<String>,
) -> bool {
    crate::tabs::creation::create(
        state,
        jobs,
        origin,
        cwd,
        name,
        crate::tabs::creation::Placement::Tab,
    )
}

pub(crate) fn enqueue_shell(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    origin: HubId,
    cwd: String,
    name: Option<String>,
) -> Option<Uuid> {
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        creation_token: Uuid::new_v4(),
        name,
        cwd,
        size: TerminalSize::default(),
    });
    match request {
        Ok(request) => {
            let id = request.fields().request_id;
            let sent = crate::creation_actions::enqueue(state, jobs, origin, request);
            if sent {
                state.status = STARTING_SHELL.into();
            }
            sent.then_some(id)
        }
        Err(error) => {
            state.status = error.to_string();
            None
        }
    }
}
