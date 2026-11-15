use super::constants::*;
use std::{path::PathBuf, sync::Arc};
use tokio::time::Instant;
use yolo_hub_agent_protocol::{Activity, Lifecycle, ObservedProvider};
use yolo_hub_connector_protocol::*;
use yolo_hub_connector_store::CONNECTOR_DIRECTORY;
use yolo_hub_identity::*;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub state: Arc<HubState>,
    pub backend: LocalBackend,
    pub connector_path: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir_in(ROOT_DIRECTORY).unwrap();
        let path = root.path().canonicalize().unwrap();
        let home = path.join(HOST_DIRECTORY);
        let state = Arc::new(HubState::open(&path.join(STATE_DIRECTORY), &home).unwrap());
        let backend = LocalBackend::new(state.hub_id(), &home, &path.join(HOST_SOCKET)).unwrap();
        let directory = state.directory().unwrap();
        let connector_path = directory.path().join(CONNECTOR_DIRECTORY);
        let backend = backend
            .with_connectors(
                directory.ensure_descendant(&connector_path).unwrap(),
                state.peer_identity(),
            )
            .unwrap();
        Self {
            root,
            state,
            backend,
            connector_path,
        }
    }
    pub fn request(&self, command: Command) -> Request {
        Request::new(self.state.hub_id(), command).unwrap()
    }
    pub fn activation(&self) -> Activation {
        let hub = self.state.hub_id();
        Activation {
            connector_id: ConnectorId::new(),
            generation: ConnectorGeneration::new(),
            agent: AgentTarget {
                address: AgentAddress {
                    hub_id: hub,
                    agent_id: AgentId::new(),
                },
                execution_id: ExecutionId::new(),
            },
            terminal: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: hub,
                    terminal_id: TerminalId::new(),
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
            next_sequence: FIRST_PUBLICATION_SEQUENCE,
        }
    }
    pub fn invalid_backend(&self) -> LocalBackend {
        let path = self.root.path().canonicalize().unwrap();
        LocalBackend::new(
            HubId::new(),
            &path.join(HOST_DIRECTORY),
            &path.join(HOST_SOCKET),
        )
        .unwrap()
    }
}
pub fn publication(activation: Activation) -> Publication {
    Publication {
        connector_id: activation.connector_id,
        generation: activation.generation,
        agent: activation.agent,
        terminal: activation.terminal,
        sequence: FIRST_PUBLICATION_SEQUENCE,
        state: ReportedState {
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            working_directory: None,
        },
    }
}
pub fn commands(activation: Activation) -> Vec<Command> {
    vec![
        Command::Hello {},
        Command::List {},
        Command::Register {
            scope: ConnectorScope {
                terminal: activation.terminal,
                provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
            },
        },
        Command::Activate {
            connector_id: activation.connector_id,
            credential: ConnectorCredential::generate(),
            instance_id: ConnectorInstanceId::new(),
        },
        Command::Publish {
            publication: publication(activation),
        },
        Command::Revoke {
            connector_id: activation.connector_id,
            expected_revision: FIRST_CONNECTOR_REVISION,
        },
    ]
}
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
