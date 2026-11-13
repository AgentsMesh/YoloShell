use super::constants::*;
use std::{path::Path, sync::Arc};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, Reply, Request, Response, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_ssh_profile_runtime::SshProfileRuntime;
use yolo_hub_ssh_profiles::{
    SshBrokerProfile, SshProfileChange, SshProfileConfiguration, SshProfileRef, SshProfileState,
};
use yolo_hub_state::HubState;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub state: Arc<HubState>,
    pub runtime: Arc<SshProfileRuntime>,
    pub dispatcher: Arc<dyn Dispatcher>,
}
pub struct Base(pub HubId);
impl Dispatcher for Base {
    fn hub_id(&self) -> HubId {
        self.0
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            match request.fields().command {
                Command::Hello {} => Ok(Operation::Reply(Reply::Hello {
                    hello: Hello {
                        hub_id: self.0,
                        version: HUB_PROTOCOL_VERSION,
                        methods: SUPPORTED_METHODS.to_vec(),
                    },
                })),
                _ => Err(PublicError::Unsupported),
            }
        })
    }
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(
            HubState::open(
                &root.path().join(HUB_DIRECTORY),
                &root.path().join(HOST_DIRECTORY),
            )
            .unwrap(),
        );
        let runtime = SshProfileRuntime::open(state.clone(), None).unwrap();
        let dispatcher = runtime.dispatcher(Arc::new(Base(state.hub_id()))).unwrap();
        Self {
            root,
            state,
            runtime,
            dispatcher,
        }
    }
    pub async fn call(&self, command: Command) -> Result<Reply, PublicError> {
        call(&self.dispatcher, command).await
    }
    pub async fn current(&self) -> SshProfileState {
        let Reply::SshProfiles { state } = self.call(Command::GetSshProfiles {}).await.unwrap()
        else {
            panic!()
        };
        state
    }
    pub fn profile(&self, broker: &Path) -> SshBrokerProfile {
        SshBrokerProfile {
            launch: None,
            profile: SshProfileRef::new(PROFILE_ID, DIGEST_CHARACTER.repeat(DIGEST_BYTES)).unwrap(),
            broker_state: broker.to_owned(),
            config_revision: DIGEST_CHARACTER.repeat(DIGEST_BYTES),
        }
    }
    pub async fn apply(&self, profiles: Vec<SshBrokerProfile>) -> SshProfileChange {
        let change = SshProfileChange {
            request_id: uuid::Uuid::new_v4(),
            expected_revision: self.current().await.revision,
            configuration: SshProfileConfiguration {
                profiles,
                ..Default::default()
            },
        };
        self.call(Command::ConfigureSshProfiles {
            change: change.clone(),
        })
        .await
        .unwrap();
        change
    }
}
pub async fn call(
    dispatcher: &Arc<dyn Dispatcher>,
    command: Command,
) -> Result<Reply, PublicError> {
    let request = Request::new(dispatcher.hub_id(), command).unwrap();
    let Operation::Reply(reply) = dispatcher.dispatch(request.clone()).await? else {
        panic!()
    };
    Response::new(&request, dispatcher.hub_id(), reply.clone())
        .validate_for(&request)
        .unwrap();
    Ok(reply)
}
pub fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + TIMEOUT
}
