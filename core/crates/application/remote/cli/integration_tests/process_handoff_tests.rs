use super::process_handoff_server::ProcessSshServer;
use super::{BrokerClient, BrokerConfig, BrokerPaths, fixture};
use std::path::PathBuf;
use yolo_client::Client;
use yolo_remote_broker::broker_instance_available;
use yolo_runtime::protocol::method;
#[path = "../test_support/process_handoff_process.rs"]
mod process;
pub(super) use process::BrokerProcess;

pub(super) struct ProcessFixture {
    _root: tempfile::TempDir,
    pub(super) config: PathBuf,
    pub(super) paths: BrokerPaths,
    pub(super) revision: String,
    _ssh: ProcessSshServer,
}

impl ProcessFixture {
    pub(super) async fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let ssh = ProcessSshServer::start(root.path()).await;
        let config = root.path().join(fixture::PROCESS_CONFIG_FILE);
        let state = root.path().join(fixture::STATE_DIRECTORY);
        let value = serde_json::json!({
            (yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD): false,
            (fixture::VERSION_FIELD): 1,
            (fixture::HOST_ID_FIELD): fixture::TEST_HOST_ID,
            (fixture::PROFILE_AUTHORITY_SHA256_FIELD): fixture::TEST_PROFILE_AUTHORITY_SHA256,
            (fixture::ENDPOINT_FIELD): {
                (fixture::USER_FIELD): fixture::TEST_USER,
                (fixture::HOST_FIELD): fixture::LOOPBACK_HOST,
                (fixture::PORT_FIELD): ssh.port,
            },
            (fixture::KNOWN_HOSTS_FIELD): ssh.known_hosts,
            (fixture::IDENTITIES_FIELD): [ssh.identity],
            (fixture::USE_AGENT_FIELD): false,
            (fixture::RUNTIME_FIELD): {
                (fixture::EXECUTABLE_FIELD): fixture::TEST_RUNTIME,
                (fixture::HOME_FIELD): fixture::TEST_HOME,
            },
            (fixture::POLICY_FIELD): {
                (fixture::ALLOWED_PROGRAMS_FIELD): [fixture::TEST_PROGRAM],
                (fixture::READ_ROOTS_FIELD): [fixture::TEST_READ_ROOT],
            },
        });
        std::fs::write(&config, serde_json::to_vec(&value).unwrap()).unwrap();
        std::fs::set_permissions(
            &config,
            std::fs::Permissions::from_mode(fixture::PRIVATE_FILE_MODE),
        )
        .unwrap();
        let revision = BrokerConfig::load(&config).unwrap().revision().to_owned();
        let paths = BrokerPaths::new(state).unwrap();
        Self {
            _root: root,
            config,
            paths,
            revision,
            _ssh: ssh,
        }
    }

    pub(super) async fn wait_ready(&self) {
        let client = BrokerClient::new(self.paths.management_socket());
        let deadline = tokio::time::Instant::now() + fixture::PROCESS_WAIT_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if client.status().await.is_ok() {
                return;
            }
            tokio::time::sleep(fixture::PROCESS_WAIT_INTERVAL).await;
        }
        panic!("broker process did not become ready");
    }

    async fn wait_released(&self) {
        let deadline = tokio::time::Instant::now() + fixture::PROCESS_WAIT_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if broker_instance_available(&self.paths).unwrap() {
                return;
            }
            tokio::time::sleep(fixture::PROCESS_WAIT_INTERVAL).await;
        }
        panic!("broker process did not release instance ownership");
    }
}

#[tokio::test]
async fn production_serve_hands_off_sockets_without_dropping_active_runtime() {
    let fixture = ProcessFixture::new().await;
    let binary =
        std::fs::canonicalize(std::env::var_os(fixture::REMOTE_BINARY_ENVIRONMENT).unwrap())
            .unwrap();
    let mut retiring = BrokerProcess::start(
        &binary,
        &fixture.config,
        fixture.paths.state_dir(),
        &fixture.revision,
    )
    .await;
    fixture.wait_ready().await;
    let mut active = Client::new(fixture.paths.runtime_socket())
        .connect()
        .await
        .unwrap();
    let _: () = active.request(method::ATTACH_SESSION, &()).await.unwrap();
    BrokerClient::new(fixture.paths.management_socket())
        .shutdown(fixture.revision.clone())
        .await
        .unwrap();
    fixture.wait_released().await;
    assert!(retiring.is_running());

    let mut replacement = BrokerProcess::start(
        &binary,
        &fixture.config,
        fixture.paths.state_dir(),
        &fixture.revision,
    )
    .await;
    fixture.wait_ready().await;
    let _: () = active.request(method::ATTACH_SESSION, &()).await.unwrap();
    assert!(retiring.is_running());
    drop(active);
    retiring.wait_for_exit().await;

    BrokerClient::new(fixture.paths.management_socket())
        .shutdown(fixture.revision)
        .await
        .unwrap();
    replacement.wait_for_exit().await;
}
