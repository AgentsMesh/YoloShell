#[path = "cleanup_registry_test_support.rs"]
mod cleanup_registry_test_support;

use super::super::constants::{MANAGEMENT_DRAIN_TIMEOUT, REMOTE_CLEANUP_DRAIN_TIMEOUT};
use super::server_support::Harness;
use super::support::{ALLOWED_PROGRAM, CONFIG_REVISION, test_config};
use crate::remote_broker::{BrokerClient, BrokerCommandRequest, BrokerRequest, BrokerResult};
use cleanup_registry_test_support::{CleanupCompletion, CleanupConnection, CleanupProbe};
use std::sync::Arc;

#[tokio::test(start_paused = true)]
async fn rotation_does_not_return_before_registered_cleanup_finishes() {
    let probe = CleanupProbe::new(CleanupCompletion::Released);
    let factory = CleanupConnection::factory(Arc::clone(&probe));
    let harness = Harness::start_with_connection_factory(factory, test_config()).await;
    let command = spawn_command(&harness);
    probe.wait_command().await;

    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    tokio::time::advance(MANAGEMENT_DRAIN_TIMEOUT).await;
    probe.wait_cleanup().await;

    assert!(!harness.task_finished());
    probe.release_cleanup();
    assert!(command.await.unwrap().is_err());
    harness.wait_stopped().await;
}

#[tokio::test(start_paused = true)]
async fn retirement_fails_closed_after_cleanup_becomes_indeterminate() {
    let probe = CleanupProbe::new(CleanupCompletion::Indeterminate);
    let factory = CleanupConnection::factory(Arc::clone(&probe));
    let harness = Harness::start_with_connection_factory(factory, test_config()).await;
    let command = spawn_command(&harness);
    probe.wait_command().await;

    let client = BrokerClient::new(harness.paths.management_socket());
    let status = client.status().await.unwrap();
    let result = client
        .request(BrokerRequest::Retire {
            expected_revision: status.config_revision,
            expected_instance_id: status.instance_id,
        })
        .await
        .unwrap();
    assert!(matches!(result, BrokerResult::Retire { .. }));
    harness.wait_socket_removed().await;
    tokio::time::advance(MANAGEMENT_DRAIN_TIMEOUT).await;
    probe.wait_cleanup().await;

    tokio::task::yield_now().await;
    tokio::time::advance(REMOTE_CLEANUP_DRAIN_TIMEOUT).await;
    tokio::task::yield_now().await;
    assert!(!harness.task_finished());
    assert!(command.await.unwrap().is_err());
    harness.abort().await;
}

#[tokio::test(start_paused = true)]
async fn rotation_fails_closed_after_registered_cleanup_fails() {
    let probe = CleanupProbe::new(CleanupCompletion::Failed);
    let factory = CleanupConnection::factory(Arc::clone(&probe));
    let harness = Harness::start_with_connection_factory(factory, test_config()).await;
    let command = spawn_command(&harness);
    probe.wait_command().await;

    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    tokio::time::advance(MANAGEMENT_DRAIN_TIMEOUT).await;
    probe.wait_cleanup().await;
    tokio::task::yield_now().await;

    assert!(!harness.task_finished());
    assert!(command.await.unwrap().is_err());
    harness.abort().await;
}

fn spawn_command(
    harness: &Harness,
) -> tokio::task::JoinHandle<Result<BrokerResult, crate::remote_broker::BrokerClientError>> {
    let client = BrokerClient::new(harness.paths.management_socket());
    tokio::spawn(async move {
        client
            .run_command(BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new()))
            .await
    })
}
