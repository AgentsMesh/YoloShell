use super::fixtures;
use crate::{
    Mode, State,
    constants::QUEUE_CAPACITY,
    events,
    network::{Job, Update},
};
use crossterm::event::{Event, KeyCode};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreateShellRequest, CreateShellResult, SecretCapability};
#[path = "creation_constants.rs"]
mod constants;
pub use constants::*;
pub struct Fixture {
    pub state: State,
    pub jobs: mpsc::Sender<Job>,
    pub incoming: mpsc::Receiver<Job>,
    pub origin: HubId,
    pub tab: Uuid,
    pub pane: Uuid,
    pub expected: TerminalAddress,
}
impl Fixture {
    pub fn new() -> Self {
        Self::with_capacity(QUEUE_CAPACITY)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        let existing = fixtures::session();
        let expected = existing.fields().address;
        let mut state = crate::client_capability_support::state();
        // Transaction fixtures address an explicit existing destination. Tab
        // allocation and user navigation are covered by tab-specific tests.
        state.inventory.view = yolo_tui_inventory::View::Terminals;
        state.control_generation = Some(Uuid::new_v4());
        state.entry_hub = expected.hub_id;
        state.tabs = yolo_tui_tab::TabCatalog::new(
            expected.hub_id,
            yolo_tui_tab::LocalTabView::default(),
        );
        state.replace_terminals(vec![existing]);
        state.bind_focused(Some(expected)).unwrap();
        let tab = state.tabs.active().id;
        let pane = state.tab_layout().focused();
        let (jobs, incoming) = mpsc::channel(capacity);
        Self {
            state,
            jobs,
            incoming,
            origin: expected.hub_id,
            tab,
            pane,
            expected,
        }
    }
    pub fn start(&mut self) -> ControlRequest {
        self.state.mode = Mode::Inventory;
        assert!(
            crate::shell_actions::enqueue_shell(
                &mut self.state,
                &self.jobs,
                self.origin,
                fixtures::SESSION_DIRECTORY.into(),
                None,
            )
            .is_some()
        );
        let Job::Call {
            generation,
            origin,
            command: Command::Create { request },
        } = self.incoming.try_recv().unwrap()
        else {
            panic!()
        };
        assert_eq!(generation, self.state.control_generation);
        assert!(
            self.state
                .creations
                .pending
                .contains_key(&request.fields().request_id)
        );
        ControlRequest {
            validity: None,
            generation,
            destination: origin,
            command: Command::Create { request },
        }
    }
    pub fn created(&self) -> CreateShellResult {
        CreateShellResult {
            address: TerminalAddress {
                hub_id: self.origin,
                terminal_id: TerminalId::new(),
            },
            creation_capability: SecretCapability::generate(),
        }
    }
    pub fn select_other(&mut self) {
        let other = fixtures::session();
        let address = other.fields().address;
        self.state.terminals.insert(address, other);
        let key = if self.state.visible().first() == Some(&self.expected) {
            KeyCode::Down
        } else {
            KeyCode::Up
        };
        crate::actions::handle(
            &mut self.state,
            &self.jobs,
            self.origin,
            Event::Key(key.into()),
        );
        assert_eq!(self.state.selected, Some(address));
    }
    pub fn finish(&mut self, request: ControlRequest, creation: CreateShellResult) {
        events::apply(
            &mut self.state,
            &self.jobs,
            Update::Control(ControlEvent::Finished {
                request,
                result: Ok(Reply::Created { creation }),
                connected: true,
            }),
        );
    }
    pub fn assert_finalize(
        &mut self,
        request: &ControlRequest,
        creation: CreateShellResult,
        expected: bool,
    ) {
        let mut finalized = false;
        while let Ok(job) = self.incoming.try_recv() {
            match job {
                Job::Refresh { .. } => {}
                Job::Finalize {
                    generation,
                    creation_token,
                    creation: actual,
                    validity,
                } => {
                    assert!(!finalized);
                    assert_eq!(Some(generation), request.generation);
                    assert_eq!(actual.address.hub_id, self.origin);
                    assert_eq!(creation_token, create(request).fields().creation_token);
                    assert_eq!(actual, creation);
                    assert!(validity.is_current());
                    finalized = true;
                }
                _ => panic!(
                    "creation completion must not replay, acquire, terminate or reassign another session"
                ),
            }
        }
        assert_eq!(finalized, expected);
    }
    pub fn switch_tab(&mut self) -> Uuid {
        self.state.tabs = self.state.tab_snapshot();
        let id = self.state.tabs.create(OTHER_TAB).unwrap();
        self.state.activate_tab_collection();
        id
    }
    pub fn original_binding(&self) -> Option<TerminalAddress> {
        if self.state.tabs.active().id == self.tab {
            return self
                .state
                .tab_layout()
                .panes()
                .into_iter()
                .find(|pane| pane.id == self.pane)
                .and_then(|pane| pane.terminal);
        }
        self.state
            .tabs
            .entries()
            .iter()
            .find(|entry| entry.id == self.tab)
            .and_then(|entry| {
                entry
                    .view
                    .layout
                    .panes()
                    .into_iter()
                    .find(|pane| pane.id == self.pane)
            })
            .and_then(|pane| pane.terminal)
    }
}

pub fn create(request: &ControlRequest) -> &CreateShellRequest {
    let Command::Create { request } = &request.command else {
        panic!()
    };
    request
}
