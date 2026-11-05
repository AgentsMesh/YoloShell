use super::hub_start_fixture::*;
use crate::pending::PendingObservation;
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::FakePending;
use crate::{Result, StartRequest, start::BrokerSpawner};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::net::UnixListener;
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::*;
use yolo_runtime::protocol::{read_frame, write_frame};

const INCREMENT: usize = 1;
const SOCKET_MODE: u32 = 0o600;
const UNEXPECTED_REQUEST: &str = "Hub readiness requested a legacy runtime operation";
pub const IDENTITY_MISMATCH: &str = "bootstrapped broker identity changed";

#[derive(Clone, Copy)]
pub enum Fault {
    None,
    Instance,
    Profile,
    Authority,
    Revision,
    StatusHost,
}

pub struct WireBroker {
    task: tokio::task::JoinHandle<Result<()>>,
    pub requests: Arc<Mutex<Vec<BrokerRequest>>>,
}
impl WireBroker {
    pub fn for_launch(launch: &ChildLaunch, adopted: bool, fault: Fault) -> Self {
        Self::bind(
            &StartRequest {
                config_source: launch.config().to_path_buf(),
                broker_state: launch.state_dir().to_path_buf(),
            },
            adopted,
            fault,
        )
    }
    pub fn bind(options: &StartRequest, adopted: bool, fault: Fault) -> Self {
        let config = BrokerConfig::load(&options.config_source).unwrap();
        let socket = BrokerPaths::new(&options.broker_state)
            .unwrap()
            .management_socket();
        let listener = UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(SOCKET_MODE)).unwrap();
        let expected = profile(&config);
        let mut status = BrokerStatus {
            protocol_version: BROKER_PROTOCOL_VERSION,
            adopted,
            instance_id: uuid::Uuid::new_v4(),
            host_id: config.host_id().into(),
            connection_id: None,
            connection_generation: u64::default(),
            config_revision: config.revision().into(),
            capabilities: vec![
                HUB_TRANSPORT_CAPABILITY.into(),
                WORKING_DIRECTORY_CAPABILITY.into(),
            ],
        };
        if matches!(fault, Fault::StatusHost) {
            status.host_id = WRONG_HOST.into();
        }
        let mut binding = BrokerHubConnectionBinding {
            profile: expected.clone(),
            broker_instance_id: status.instance_id,
            connection_id: uuid::Uuid::new_v4(),
            connection_generation: GENERATION,
        };
        match fault {
            Fault::Instance => binding.broker_instance_id = uuid::Uuid::new_v4(),
            Fault::Profile => binding.profile.host_id = WRONG_HOST.into(),
            Fault::Authority => binding.profile.profile_authority_sha256 = WRONG_REVISION.into(),
            Fault::Revision => binding.profile.config_revision = WRONG_REVISION.into(),
            _ => {}
        }
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await?;
                let request: BrokerRequestEnvelope = read_frame(&mut stream).await?;
                recorded.lock().unwrap().push(request.request.clone());
                let result = match request.request {
                    BrokerRequest::Status => BrokerResult::Status(status.clone()),
                    BrokerRequest::PrepareHubTransport(actual) => {
                        anyhow::ensure!(actual == expected, UNEXPECTED_REQUEST);
                        BrokerResult::HubTransport(binding.clone())
                    }
                    _ => anyhow::bail!(UNEXPECTED_REQUEST),
                };
                write_frame(
                    &mut stream,
                    &BrokerResponse {
                        protocol_version: BROKER_PROTOCOL_VERSION,
                        request_id: request.request_id,
                        result: Some(result),
                        error_code: None,
                        error_message: None,
                    },
                )
                .await?;
            }
        });
        Self { task, requests }
    }
}
impl Drop for WireBroker {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub struct PendingSpawner {
    fault: Fault,
    pub calls: AtomicUsize,
    pub compensated: Arc<AtomicUsize>,
    pub servers: Mutex<Vec<WireBroker>>,
}
impl PendingSpawner {
    pub fn new(fault: Fault) -> Self {
        Self {
            fault,
            calls: AtomicUsize::default(),
            compensated: Arc::new(AtomicUsize::default()),
            servers: Mutex::new(Vec::new()),
        }
    }
}
impl BrokerSpawner for PendingSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.calls.fetch_add(INCREMENT, Ordering::SeqCst);
        self.servers
            .lock()
            .unwrap()
            .push(WireBroker::for_launch(options, false, self.fault));
        let compensated = self.compensated.clone();
        Ok(FakePending::tracked_pending(
            || Ok(PendingObservation::Pending),
            move || {
                compensated.fetch_add(INCREMENT, Ordering::SeqCst);
                Ok(())
            },
        ))
    }
}

#[derive(Default)]
pub struct NoSpawn(pub AtomicUsize);
impl BrokerSpawner for NoSpawn {
    fn spawn(&self, _: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.0.fetch_add(INCREMENT, Ordering::SeqCst);
        anyhow::bail!(UNEXPECTED_REQUEST)
    }
}
