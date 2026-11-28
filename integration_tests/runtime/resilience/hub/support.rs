use crate::Harness;
use std::path::PathBuf;
use tokio::{sync::oneshot, task::JoinHandle, time::Instant};
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_server::HubServer;
use yolo_hub_terminal_protocol::*;
use yolo_hub_state::HubState;

#[path = "constants.rs"]
mod constants;
pub use constants::STATE_DIRECTORY;

pub const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);
pub const SOCKET_TEST_ROOT: &str = "/tmp";
pub const SESSION_NAME: &str = "Hub public protocol E2E";
pub const OUTPUT_COMMAND: &[u8] = b"printf 'YOLO_HUB_%s\\n' VERIFIED\n";
pub const OUTPUT_MARKER: &str = "YOLO_HUB_VERIFIED";
pub const EXIT_COMMAND: &[u8] = b"exit\n";
pub const REJECTED_COMMAND: &[u8] = b"printf 'REVOKED_INPUT_EXECUTED\\n'\n";
pub const MAX_OBSERVED_BYTES: usize = 1024 * 1024;
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}

pub struct RunningHub {
    pub hub: HubId,
    pub socket: PathBuf,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl RunningHub {
    pub async fn start(harness: &Harness, path: &std::path::Path) -> Self {
        let state = HubState::open(path, harness.home.path()).unwrap();
        let hub = state.hub_id();
        let socket = state.socket_path();
        let backend = super::fixture_agents::with_fixture_agents(
            LocalBackend::new(
                hub,
                harness.home.path(),
                &harness.paths().host_socket().unwrap(),
            )
            .unwrap(),
            &state,
        );
        let host_hold = backend.retain_host().await.unwrap_or_else(|error| {
            panic!(
                "Hub fixture retain failed: {error:?}; host={:?}; alive={}; stderr={}",
                harness.host_identity(),
                harness.host_identity().matches(),
                harness.host_stderr_tail()
            )
        });
        assert_eq!(host_hold.initial_host_pid(), harness.host_identity().pid);
        assert!(harness.host_identity().matches());
        let server = HubServer::bind(state, backend).await.unwrap();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _host_hold = host_hold;
            server
                .serve_until(async {
                    let _ = stopped.await;
                })
                .await
        });
        Self {
            hub,
            socket,
            stop: Some(stop),
            task: Some(task),
        }
    }

    pub async fn client(&self) -> yolo_hub_client::LocalClient {
        Client::connect_local(&self.socket, self.hub, deadline())
            .await
            .unwrap()
    }

    pub async fn stop(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(TEST_TIMEOUT, self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}

impl Drop for RunningHub {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

pub async fn create(hub: &RunningHub, harness: &Harness) -> TerminalSummary {
    let mut client = hub.client().await;
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
    let Reply::Created { creation } = client
        .call(hub.hub, Command::Create { request }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let view = harness
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    harness.track_view(&view);
    client
        .call(
            hub.hub,
            Command::Finalize {
                creation_token: token,
                creation,
            },
            deadline(),
        )
        .await
        .unwrap();
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
    session
}

pub fn input(grant: LeaseGrant, data: &[u8]) -> Command {
    Command::Input {
        request: InputRequest::new(InputFields {
            request_id: uuid::Uuid::new_v4(),
            grant,
            data: data.to_vec(),
        })
        .unwrap(),
    }
}

pub fn acquire(target: yolo_hub_identity::RuntimeTarget, mode: LeaseMode) -> Command {
    Command::Lease {
        request: LeaseRequest {
            request_id: uuid::Uuid::new_v4(),
            target,
            principal: WriterPrincipal {
                client_id: uuid::Uuid::new_v4(),
                controller_instance_id: uuid::Uuid::new_v4(),
            },
            mode,
        },
    }
}
