use super::interrupt_support;
use crate::{Mode, State, actions, network::Job};
use crossterm::event::{Event, KeyCode, KeyModifiers};
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{Activity, Capability};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_terminal_protocol::{TerminalSize, TerminalSnapshot, TerminalSnapshotFields};
#[path = "chat_constants.rs"]
mod constants;
pub use constants::*;

pub fn ready() -> (State, mpsc::Sender<Job>, mpsc::Receiver<Job>) {
    let mut state = interrupt_support::ready();
    state.mode = Mode::Terminal;
    interrupt_support::mutate_agent(&mut state, |fields| {
        fields.activity = Activity::WaitingForInput;
        fields.capabilities.submit_prompt = Capability::Available {};
    });
    let address = state.focused_terminal().unwrap();
    let target = state.terminals[&address].fields().runtime.unwrap().target;
    let size = TerminalSize::default();
    state.set_last_size(address, size);
    state.set_observer_generation(address, uuid::Uuid::new_v4());
    state.surfaces.insert(
        address,
        yolo_tui_terminal::TerminalSurface::new(
            &TerminalSnapshot::new(TerminalSnapshotFields {
                target,
                barrier_sequence: 0,
                size,
                data: PASTE_MODE.to_vec(),
            })
            .unwrap(),
        ),
    );
    let (jobs, incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    (state, jobs, incoming)
}
pub fn key(state: &mut State, jobs: &mpsc::Sender<Job>, code: KeyCode, modifiers: KeyModifiers) {
    assert!(!actions::handle(
        state,
        jobs,
        yolo_hub_identity::HubId::new(),
        Event::Key(crossterm::event::KeyEvent::new(code, modifiers))
    ));
}
pub fn toggle(state: &mut State, jobs: &mpsc::Sender<Job>) {
    key(state, jobs, KeyCode::Char('b'), KeyModifiers::CONTROL);
    key(
        state,
        jobs,
        KeyCode::Char(crate::chat::TOGGLE_KEY),
        KeyModifiers::NONE,
    );
}
pub fn paste(state: &mut State, jobs: &mpsc::Sender<Job>, text: &str) {
    assert!(!actions::handle(
        state,
        jobs,
        yolo_hub_identity::HubId::new(),
        Event::Paste(text.into())
    ));
}
pub fn sent(incoming: &mut mpsc::Receiver<Job>) -> ControlRequest {
    let Job::Scoped(request) = incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}
