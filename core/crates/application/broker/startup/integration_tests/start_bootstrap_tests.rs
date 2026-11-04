use super::super::{BrokerConfig, BrokerPaths, Result};
use super::fixture;
use super::support::{RuntimeFixture, running_fixture_with, shutdown_running_fixture};
use crate::pending::PendingSpawnHandle;
use crate::pending_fixture::{FakePending, TrackedBrokerTask, TrackedTaskSettlement};
use crate::start::{BrokerSpawner, reconcile};
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::net::{UnixListener, UnixStream};
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BrokerLaunchDeadline, BrokerRequest, BrokerRequestEnvelope,
    BrokerResponse, BrokerResult, BrokerStatus, CONNECTION_BOOTSTRAP_CAPABILITY,
    WORKING_DIRECTORY_CAPABILITY,
};
use yolo_runtime::protocol::{read_frame, write_frame};

const EXPECTED_SPAWN_ATTEMPTS: usize = 1;
const SPAWN_ATTEMPT_INCREMENT: usize = 1;
const PENDING_BOOTSTRAP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
const EXISTING_BOOTSTRAP_FAILURE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const UNEXPECTED_SPAWN: &str = "existing broker bootstrap failure attempted a replacement";
const STATUS_REQUEST_REQUIRED: &str = "pending broker expected a status request";
const BOOTSTRAP_REQUEST_REQUIRED: &str = "pending broker expected a bootstrap request";

#[derive(Default)]
struct NoSpawn(AtomicUsize);

impl BrokerSpawner for NoSpawn {
    fn spawn(&self, _: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.0.fetch_add(SPAWN_ATTEMPT_INCREMENT, Ordering::Relaxed);
        anyhow::bail!(UNEXPECTED_SPAWN)
    }
}

#[derive(Default)]
struct PendingSpawner {
    calls: AtomicUsize,
    tasks: Mutex<Vec<TrackedBrokerTask>>,
}

impl BrokerSpawner for PendingSpawner {
    fn spawn(&self, options: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.calls
            .fetch_add(SPAWN_ATTEMPT_INCREMENT, Ordering::Relaxed);
        let config = BrokerConfig::load(&options.config())?;
        let socket = BrokerPaths::new(&options.state_dir())?.management_socket();
        let listener = UnixListener::bind(socket.as_path())?;
        std::fs::set_permissions(
            &socket,
            std::fs::Permissions::from_mode(fixture::PRIVATE_FILE_MODE),
        )?;
        let status = pending_status(&config);
        let (pending, task) = FakePending::tracked_task(serve_pending(listener, status));
        self.tasks.lock().unwrap().push(task);
        Ok(pending)
    }
}

impl PendingSpawner {
    async fn assert_compensated(&self) {
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        assert_eq!(tasks.len(), EXPECTED_SPAWN_ATTEMPTS);
        for task in tasks {
            assert!(matches!(
                task.settle().await.unwrap(),
                TrackedTaskSettlement::Cancelled
            ));
        }
    }
}

fn pending_status(config: &BrokerConfig) -> BrokerStatus {
    BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: false,
        instance_id: uuid::Uuid::new_v4(),
        host_id: config.host_id().into(),
        connection_id: None,
        connection_generation: u64::default(),
        config_revision: config.revision().into(),
        capabilities: vec![
            CONNECTION_BOOTSTRAP_CAPABILITY.into(),
            WORKING_DIRECTORY_CAPABILITY.into(),
        ],
    }
}

async fn serve_pending(listener: UnixListener, status: BrokerStatus) -> Result<()> {
    let mut status_stream = accept(&listener).await?;
    let status_request: BrokerRequestEnvelope = read_frame(&mut status_stream).await?;
    anyhow::ensure!(
        matches!(status_request.request, BrokerRequest::Status),
        STATUS_REQUEST_REQUIRED
    );
    let response = BrokerResponse {
        protocol_version: BROKER_PROTOCOL_VERSION,
        request_id: status_request.request_id,
        result: Some(BrokerResult::Status(status)),
        error_code: None,
        error_message: None,
    };
    write_frame(&mut status_stream, &response).await?;
    drop(status_stream);

    let mut bootstrap_stream = accept(&listener).await?;
    let bootstrap: BrokerRequestEnvelope = read_frame(&mut bootstrap_stream).await?;
    anyhow::ensure!(
        matches!(bootstrap.request, BrokerRequest::Bootstrap),
        BOOTSTRAP_REQUEST_REQUIRED
    );
    std::future::pending::<Result<()>>().await
}

async fn accept(listener: &UnixListener) -> Result<UnixStream> {
    listener
        .accept()
        .await
        .map(|value| value.0)
        .map_err(Into::into)
}

#[tokio::test]
async fn existing_broker_bootstrap_failure_leaves_the_broker_alive() {
    let (fixture, task) =
        running_fixture_with(super::ready_remote::authentication_failure_factory()).await;
    let spawner = NoSpawn::default();

    let error = reconcile(
        fixture.options.clone(),
        &spawner,
        EXISTING_BOOTSTRAP_FAILURE_TIMEOUT,
    )
    .await
    .unwrap_err();

    let detail = format!("{error:#}");
    assert!(
        detail.contains(fixture::TEST_AUTHENTICATION_FAILURE),
        "{detail}"
    );
    assert_eq!(spawner.0.load(Ordering::Relaxed), usize::default());
    assert!(!task.is_finished());
    shutdown_running_fixture(&fixture, task).await;
}

#[tokio::test(start_paused = true)]
async fn pending_bootstrap_timeout_compensates_before_adoption() {
    // Bind the process-wide anchor to real time without changing the paused timer origin.
    std::thread::spawn(|| {
        assert!(tokio::runtime::Handle::try_current().is_err());
        BrokerLaunchDeadline::capture(tokio::time::Instant::now() + fixture::START_TIMEOUT)
    })
    .join()
    .unwrap()
    .unwrap();
    let fixture = RuntimeFixture::new();
    let spawner = PendingSpawner::default();
    let started = tokio::time::Instant::now();

    reconcile(fixture.options.clone(), &spawner, PENDING_BOOTSTRAP_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(
        spawner.calls.load(Ordering::Relaxed),
        EXPECTED_SPAWN_ATTEMPTS
    );
    assert_eq!(
        tokio::time::Instant::now(),
        started + PENDING_BOOTSTRAP_TIMEOUT
    );
    spawner.assert_compensated().await;
}
