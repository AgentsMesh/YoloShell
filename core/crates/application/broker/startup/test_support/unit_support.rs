use super::fixture;
use super::{Result, StartRequest};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};
use yolo_remote_broker::{BrokerClient, BrokerConfig, BrokerPaths};

pub struct RuntimeFixture {
    _root: tempfile::TempDir,
    pub options: StartRequest,
}

impl RuntimeFixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let known_hosts = root.path().join(fixture::KNOWN_HOSTS_FILE);
        fs::write(&known_hosts, fixture::TEST_KNOWN_HOST).unwrap();
        fs::set_permissions(&known_hosts, fs::Permissions::from_mode(0o644)).unwrap();
        let mut config = serde_json::json!({
            (yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD): false,
            (fixture::VERSION_FIELD): 1,
            (fixture::HOST_ID_FIELD): fixture::TEST_HOST_ID,
            (fixture::PROFILE_AUTHORITY_SHA256_FIELD): fixture::TEST_PROFILE_AUTHORITY_SHA256,
            (fixture::ENDPOINT_FIELD): {
                (fixture::USER_FIELD): fixture::TEST_USER,
                (fixture::HOST_FIELD): fixture::TEST_HOST,
                (fixture::PORT_FIELD): fixture::TEST_PORT
            },
            (fixture::IDENTITIES_FIELD): [],
            (fixture::USE_AGENT_FIELD): false,
            (fixture::RUNTIME_FIELD): {
                (fixture::EXECUTABLE_FIELD): fixture::TEST_RUNTIME,
                (fixture::HOME_FIELD): fixture::TEST_HOME
            },
            (fixture::POLICY_FIELD): {
                (fixture::ALLOWED_PROGRAMS_FIELD): [fixture::TEST_PROGRAM],
                (fixture::READ_ROOTS_FIELD): [fixture::TEST_READ_ROOT]
            }
        });
        config[fixture::KNOWN_HOSTS_FIELD] = serde_json::json!(known_hosts);
        let config_path = root.path().join(fixture::CONFIG_FILE);
        fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
        fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
        let options = StartRequest {
            config_source: config_path,
            broker_state: root.path().join(fixture::STATE_DIRECTORY),
        };
        Self {
            _root: root,
            options,
        }
    }
}

pub async fn running_fixture() -> (RuntimeFixture, tokio::task::JoinHandle<Result<()>>) {
    running_fixture_with(super::ready_remote::factory()).await
}

pub async fn running_fixture_with(
    factory: std::sync::Arc<dyn yolo_remote_broker::BrokerConnectionFactory>,
) -> (RuntimeFixture, tokio::task::JoinHandle<Result<()>>) {
    let fixture = RuntimeFixture::new();
    let launch = authorize(&fixture.options).await;
    let expected_revision = launch.config_revision().to_owned();
    let paths = BrokerPaths::open_existing(fixture.options.broker_state.clone()).unwrap();
    let task = tokio::spawn(serve_with_remote(launch, factory));
    let socket = paths.management_socket();
    let client = BrokerClient::new(socket);
    let deadline = tokio::time::Instant::now() + fixture::START_TIMEOUT;
    loop {
        if let Ok(Ok(status)) = tokio::time::timeout_at(deadline, client.status()).await
            && status.config_revision == expected_revision
        {
            return (fixture, task);
        }
        assert!(!task.is_finished());
        assert!(tokio::time::Instant::now() < deadline);
        tokio::time::sleep(yolo_remote_broker::BROKER_READY_POLL_INTERVAL).await;
    }
}

pub async fn authorize(request: &StartRequest) -> ChildLaunch {
    let expected_revision = BrokerConfig::load(&request.config_source)
        .unwrap()
        .revision()
        .to_owned();
    let paths = BrokerPaths::new(&request.broker_state).unwrap();
    let launch_deadline = tokio::time::Instant::now() + fixture::START_TIMEOUT;
    let guard = yolo_remote_broker::BrokerReconcileGuard::acquire(&paths, launch_deadline)
        .await
        .unwrap();
    let authority = yolo_remote_broker::authorize_broker_launch(&paths, &guard, launch_deadline)
        .await
        .unwrap();
    let launch = ChildLaunch::new(
        request.config_source.clone(),
        request.broker_state.clone(),
        expected_revision,
        paths.state_anchor(),
        authority,
        yolo_remote_broker::BrokerLaunchDeadline::capture(launch_deadline).unwrap(),
    )
    .unwrap();
    drop(guard);
    launch
}

pub async fn serve_ready(options: ChildLaunch) -> Result<()> {
    serve_with_remote(options, super::ready_remote::factory()).await
}

pub async fn serve_with_remote(
    options: ChildLaunch,
    factory: std::sync::Arc<dyn yolo_remote_broker::BrokerConnectionFactory>,
) -> Result<()> {
    yolo_broker_supervision::serve_with_factory(options, CredentialSource::None, factory).await
}

pub async fn shutdown_running_fixture(
    fixture: &RuntimeFixture,
    task: tokio::task::JoinHandle<Result<()>>,
) {
    let config = BrokerConfig::load(&fixture.options.config_source).unwrap();
    let socket = BrokerPaths::new(&fixture.options.broker_state)
        .unwrap()
        .management_socket();
    BrokerClient::new(socket)
        .shutdown(config.revision().into())
        .await
        .unwrap();
    task.await.unwrap().unwrap();
}
