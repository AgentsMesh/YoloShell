use crate::remote::*;
use crate::remote_broker::*;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    sync::oneshot,
    time::{Instant, timeout_at},
};

pub const PAYLOAD: &[u8] = b"Hub tunnel data\0\r\n";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const POLL: std::time::Duration = std::time::Duration::from_millis(5);
pub const ONE_TUNNEL: usize = 1;
pub const WRONG_SOCKET: &str = "/run/user/1000/yolo/wrong.sock";
const RUNTIME: &str = "/opt/yolo";
const REMOTE_HOME: &str = "/srv/yolo";
const USER: &str = "builder";
const REMOTE: &str = "build.example";
const HOST_KEY: &str = "build.example ssh-ed25519 encoded-key";
const STATE: &str = "broker";
const CONNECT_INCREMENT: usize = 1;

pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
pub fn config() -> BrokerConfig {
    let profile = yolo_broker_hub_test_support::profile();
    BrokerConfig {
        hub_transport: true,
        host_id: profile.host_id,
        profile_authority_sha256: profile.profile_authority_sha256,
        endpoint: RemoteEndpoint::standard(USER, REMOTE).unwrap(),
        known_hosts: KnownHosts::parse(HOST_KEY).unwrap(),
        identities: Vec::new(),
        agent_socket: None,
        runtime: RemoteRuntimeSpec::new(RUNTIME, REMOTE_HOME).unwrap(),
        command_policy: CommandPolicy::new([BROKER_COMMAND_CANCELLATION_PROGRAM.into()]).unwrap(),
        file_policy: FileReadPolicy::standard(vec![PathBuf::from(REMOTE_HOME)]).unwrap(),
        artifact: None,
        revision: profile.config_revision,
    }
}

pub struct Factory {
    connection: Arc<RusshConnection>,
    pub calls: AtomicUsize,
}
impl BrokerConnectionFactory for Factory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        self.calls.fetch_add(CONNECT_INCREMENT, Ordering::Relaxed);
        let connection = self.connection.clone();
        Box::pin(async move { Ok(connection as Arc<dyn MultiplexedSshConnection>) })
    }
}

pub struct Harness {
    _root: tempfile::TempDir,
    pub paths: BrokerPaths,
    pub ssh: BrokerCommandCleanupFixture,
    pub target: HubPeerTarget,
    pub factory: Arc<Factory>,
    pub pool: Arc<BrokerConnectionPool>,
    pub adoption: Option<oneshot::Sender<BrokerAdoptionAcknowledgement>>,
    stop: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Harness {
    pub async fn start() -> Self {
        Self::start_mode(
            false,
            crate::remote_broker::constants::MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }
    pub async fn pending() -> Self {
        Self::start_mode(
            true,
            crate::remote_broker::constants::MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }
    pub async fn single_management_slot() -> Self {
        Self::start_mode(false, ONE_TUNNEL).await
    }
    async fn start_mode(pending: bool, management_limit: usize) -> Self {
        let ssh = BrokerCommandCleanupFixture::connect().await;
        let target = ssh.enable_hub_peer().await;
        let factory = Arc::new(Factory {
            connection: ssh.connection(),
            calls: AtomicUsize::default(),
        });
        let root = tempfile::tempdir().unwrap();
        let paths = BrokerPaths::new(root.path().join(STATE)).unwrap();
        let broker = RemoteBroker::with_factory(config(), paths.clone(), factory.clone());
        let pool = broker.pool().clone();
        let (stop, receiver) = oneshot::channel();
        let (adoption, adoption_receive) = oneshot::channel();
        let task = tokio::spawn(async move {
            let shutdown = async {
                let _ = receiver.await;
            };
            if pending {
                broker
                    .serve_supervised_until(adoption_receive, shutdown)
                    .await
            } else {
                broker
                    .serve_until_with_limits(
                        shutdown,
                        crate::remote_broker::constants::MAX_RUNTIME_CLIENTS,
                        management_limit,
                    )
                    .await
            }
        });
        timeout_at(deadline(), async {
            while !paths.management_socket().exists() {
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        Self {
            _root: root,
            paths,
            ssh,
            target,
            factory,
            pool,
            adoption: pending.then_some(adoption),
            stop: Some(stop),
            task,
        }
    }
    pub fn client(&self) -> BrokerClient {
        BrokerClient::new(self.paths.management_socket())
    }
    pub async fn request(&self) -> BrokerHubOpenRequest {
        let mut request = yolo_broker_hub_test_support::request();
        request.binding = self
            .client()
            .prepare_hub_transport(yolo_broker_hub_test_support::profile(), deadline())
            .await
            .unwrap();
        request.socket = self.target.path.clone();
        request
    }
    pub async fn stop(mut self) {
        let _ = self.stop.take().unwrap().send(());
        timeout_at(deadline(), &mut self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        self.ssh.settle().await;
    }
    pub async fn retired(mut self) {
        timeout_at(deadline(), &mut self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        self.ssh.settle().await;
        self.stop.take();
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}
