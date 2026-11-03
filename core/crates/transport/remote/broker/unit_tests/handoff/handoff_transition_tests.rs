use super::super::constants::{BROKER_MANAGEMENT_ENDPOINT_MISSING, PRIVATE_FILE_MODE};
use super::super::*;
use super::server_support::Harness;
use super::support::{CONFIG_REVISION, FakeFactory};
use std::os::unix::fs::PermissionsExt;

const TRANSITION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;

#[tokio::test]
async fn captured_retirement_reports_a_current_live_broker() {
    let harness = Harness::start(FakeFactory::empty(Vec::new())).await;
    let guard = BrokerReconcileGuard::acquire(
        &harness.paths,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    let client = BrokerClient::new(harness.paths.management_socket());

    let transition = retirement
        .transition(
            &client,
            CONFIG_REVISION,
            &guard,
            tokio::time::Instant::now() + TRANSITION_TIMEOUT,
        )
        .await
        .unwrap();

    assert!(matches!(transition, BrokerTransition::Current(_)));
    drop(retirement);
    drop(guard);
    harness.stop().await;
}

#[tokio::test]
async fn transition_rejects_a_capture_without_a_management_endpoint() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    let client = BrokerClient::new(paths.management_socket());

    let error = retirement
        .transition(
            &client,
            CONFIG_REVISION,
            &guard,
            tokio::time::Instant::now() + TRANSITION_TIMEOUT,
        )
        .await
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(BROKER_MANAGEMENT_ENDPOINT_MISSING)
    );
}

#[tokio::test]
async fn transition_propagates_client_failure_after_the_captured_endpoint_exits() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let listener = std::os::unix::net::UnixListener::bind(paths.management_socket()).unwrap();
    std::fs::set_permissions(
        paths.management_socket(),
        std::fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
    let retirement = BrokerRetirement::capture(&guard).unwrap();
    let client = BrokerClient::new(paths.management_socket());
    drop(listener);

    let result = retirement
        .transition(
            &client,
            CONFIG_REVISION,
            &guard,
            tokio::time::Instant::now() + TRANSITION_TIMEOUT,
        )
        .await;

    assert!(result.is_err());
    guard.verify().unwrap();
}
