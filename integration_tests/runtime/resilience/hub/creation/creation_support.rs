use super::{
    public_client_support,
    support::{RunningHub, SESSION_NAME, deadline},
};
use crate::Harness;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, TERMINAL_PROTOCOL_SCHEMA_VERSION,
    TerminalSize, TerminalSummary,
};

pub struct PendingShell {
    pub token: uuid::Uuid,
    pub creation: CreateShellResult,
    pub session: TerminalSummary,
}

#[derive(Debug)]
pub enum ConnectionPhase {
    CreateAbandoned,
    CreateRetained,
    BeforeRestart,
    AfterRestart,
}

pub async fn connect(
    hub: &RunningHub,
    harness: &Harness,
    phase: ConnectionPhase,
) -> yolo_hub_client::LocalClient {
    yolo_hub_client::Client::connect_local(&hub.socket, hub.hub, deadline())
        .await.unwrap_or_else(|error| panic!(
            "cleanup Hub connection failed: phase={phase:?}; error={error:?}; host={:?}; alive={}; stderr={}",
            harness.host_identity(), harness.host_identity().matches(), harness.host_stderr_tail()
        ))
}

pub async fn pending_shell(
    hub: &RunningHub,
    harness: &Harness,
    phase: ConnectionPhase,
) -> PendingShell {
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(SESSION_NAME.into()),
        cwd: harness.home.path().display().to_string(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let mut client = connect(hub, harness, phase).await;
    let Reply::Created { creation } =
        public_client_support::call(&hub.socket, hub.hub, hub.hub, Command::Create { request }).await
    else {
        panic!()
    };
    let view = harness
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    harness.track_view(&view);
    let Reply::Terminal { terminal: session } = client
        .call(
            hub.hub,
            Command::Get {
                address: creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    PendingShell {
        token,
        creation,
        session,
    }
}
