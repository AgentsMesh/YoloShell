use super::fixtures;
use crate::{
    State, actions,
    network::{Job, Update},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{
    AgentControlRequest, AgentSnapshot, Capability, CommandOutcome, CommandReceipt,
};
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_rpc_protocol::{Command, Reply};

pub fn ready() -> State {
    let (agent, session) = fixtures::agent();
    let mut fields = agent.fields().clone();
    fields.capabilities.interrupt = Capability::Available {};
    let agent = AgentSnapshot::new(fields).unwrap();
    let target = agent.cursor().target();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.inventory.replace(Some(vec![agent]), Some(vec![]));
    state.inventory.current = true;
    state.inventory.view = yolo_tui_inventory::View::Tree;
    state.replace_terminals(vec![session.clone()]);
    state.bind_focused(Some(session.fields().address)).unwrap();
    state.inventory.select_tree(
        yolo_tui_inventory::TreeSelection::Agent(target),
        "",
        &state.terminals,
    );
    let grant = fixtures::grant(&session);
    state.principal = grant.fields().principal;
    state.record_grant(grant, Instant::now());
    state
}
pub fn target(state: &State) -> AgentTarget {
    state
        .inventory
        .agents
        .values()
        .next()
        .unwrap()
        .cursor()
        .target()
}
pub fn press(state: &mut State, jobs: &mpsc::Sender<Job>, kind: KeyEventKind) {
    let mut key = KeyEvent::new(
        KeyCode::Char(crate::interrupt_constants::KEY),
        KeyModifiers::NONE,
    );
    key.kind = kind;
    // The connected entry may differ from the selected Agent's origin.
    assert!(!actions::handle(state, jobs, HubId::new(), Event::Key(key)));
}
pub fn queued(incoming: &mut mpsc::Receiver<Job>) -> ControlRequest {
    let Job::Call {
        generation,
        origin,
        command,
    } = incoming.try_recv().unwrap()
    else {
        panic!()
    };
    ControlRequest {
        validity: None,
        generation,
        destination: origin,
        command,
    }
}
pub fn request(queued: &ControlRequest) -> &AgentControlRequest {
    let Command::AgentControl { request, .. } = &queued.command else {
        panic!()
    };
    request
}
pub fn reply(queued: &ControlRequest, outcome: CommandOutcome) -> Reply {
    let fields = request(queued).fields();
    let mut receipt = CommandReceipt::accepted(fields.request_id, fields.target).unwrap();
    receipt.advance(outcome).unwrap();
    Reply::AgentControl { receipt }
}
pub fn finish(state: &mut State, jobs: &mpsc::Sender<Job>, request: ControlRequest, reply: Reply) {
    crate::events::apply(
        state,
        jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result: Ok(reply),
            connected: true,
        }),
    );
}
pub fn mutate_agent(
    state: &mut State,
    change: impl FnOnce(&mut yolo_hub_agent_protocol::AgentSnapshotFields),
) {
    let id = state.inventory.selected_agent.unwrap();
    let mut fields = state.inventory.agents[&id].fields().clone();
    change(&mut fields);
    state
        .inventory
        .agents
        .insert(id, AgentSnapshot::new(fields).unwrap());
}
pub const SINGLE_JOB: usize = 1;
pub const PRIVATE_MESSAGE: &str = "old-channel-failure";
pub const SCREEN_COLUMNS: u16 = 120;
pub const SCREEN_ROWS: u16 = 24;
