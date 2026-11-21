use crate::{StartupAction, State, network::Job, startup};
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{CreateShellRequest, LeaseMode, RuntimeStatus, TerminalSummary};

pub const DIRECTORY: &str = "/tmp/yolo-entry-project";
pub const NAME: &str = "build";
pub const MISSING: &str = "missing-shell";

pub struct Fixture {
    pub state: State,
    pub hub: HubId,
    pub jobs: mpsc::Sender<Job>,
    pub incoming: mpsc::Receiver<Job>,
}
impl Fixture {
    pub fn new(action: StartupAction) -> Self {
        let mut state = crate::client_capability_support::state();
        let hub = HubId::new();
        startup::install(&mut state, hub, action);
        let (jobs, incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        Self {
            state,
            hub,
            jobs,
            incoming,
        }
    }
    pub fn automatic() -> Self {
        Self::new(StartupAction::Automatic {
            cwd: DIRECTORY.into(),
        })
    }
    pub fn ready(&mut self) {
        self.state.inventory.current = true;
        self.state.control_generation = Some(uuid::Uuid::new_v4());
    }
    pub fn apply(&mut self) {
        startup::apply(&mut self.state, &self.jobs);
    }
    pub fn create(&mut self) -> CreateShellRequest {
        let Job::Call {
            origin,
            command: Command::Create { request },
            ..
        } = self.incoming.try_recv().unwrap()
        else {
            panic!("expected one shell creation")
        };
        assert_eq!(origin, self.hub);
        request
    }
    pub fn acquired(&mut self, session: &TerminalSummary) {
        let Job::InitialAcquire {
            origin, request, ..
        } = self.incoming.try_recv().unwrap()
        else {
            panic!("expected scoped input acquisition")
        };
        assert_eq!(origin, session.fields().address.hub_id);
        assert_eq!(request.target, session.fields().runtime.unwrap().target);
        assert_eq!(request.mode, LeaseMode::Acquire {});
        self.no_jobs();
    }
    pub fn no_jobs(&mut self) {
        assert!(self.incoming.try_recv().is_err());
    }
}

pub fn session(hub: HubId, status: RuntimeStatus) -> TerminalSummary {
    let mut fields = super::fixtures::session().fields().clone();
    fields.address.hub_id = hub;
    fields.name = NAME.into();
    let runtime = fields.runtime.as_mut().unwrap();
    runtime.target.address = fields.address;
    runtime.status = status;
    TerminalSummary::new(fields).unwrap()
}
