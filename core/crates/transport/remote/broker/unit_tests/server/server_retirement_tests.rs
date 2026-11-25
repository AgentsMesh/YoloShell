use super::super::*;
use super::server_support::Harness;
use super::support::*;
use crate::Client;
use crate::protocol::method;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

const WRONG_HOST: &str = "wrong-retirement-host";
const CLOSED_STREAM_PROBE_BYTES: usize = 1;

#[tokio::test(start_paused = true)]
async fn credential_retirement_bounds_a_hanging_runtime_client() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let mut active = Client::new(harness.paths.runtime_socket())
        .connect()
        .await
        .unwrap();
    let _: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();

    let paths = harness.paths.clone();
    let retirement = tokio::spawn(async move {
        retire_broker(
            &paths,
            HOST_ID,
            tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
        )
        .await
    });
    harness.wait_socket_removed().await;
    assert!(!harness.task_finished());
    assert!(!broker_instance_available(&harness.paths).unwrap());
    tokio::time::advance(super::super::constants::EXTERNAL_RUNTIME_DRAIN_TIMEOUT).await;
    drop(_clock);
    retirement.await.unwrap().unwrap();
    harness.wait_stopped().await;

    let closed: Result<ProxyReply, _> = active.request(method::HOST_STATUS, &()).await;
    assert!(closed.is_err());
}

#[tokio::test]
async fn disconnected_broker_retires_without_opening_ssh() {
    let factory = FakeFactory::empty(Vec::new());
    let harness = Harness::start(factory.clone()).await;

    retire_broker(
        &harness.paths,
        HOST_ID,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();
    harness.wait_stopped().await;

    assert_eq!(factory.calls(), 0);
}

#[tokio::test]
async fn host_mismatch_does_not_retire_the_observed_instance() {
    let harness = Harness::start(FakeFactory::empty(Vec::new())).await;
    let client = BrokerClient::new(harness.paths.management_socket());

    assert!(
        retire_broker(
            &harness.paths,
            WRONG_HOST,
            tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
        )
        .await
        .is_err()
    );
    assert_eq!(client.status().await.unwrap().host_id, HOST_ID);
    harness.stop().await;
}

#[tokio::test(start_paused = true)]
async fn credential_retirement_cancels_a_silent_management_client_before_unlocking() {
    let harness = Harness::start(FakeFactory::empty(Vec::new())).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let mut silent = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    tokio::task::yield_now().await;
    let paths = harness.paths.clone();
    let retirement = tokio::spawn(async move {
        retire_broker(
            &paths,
            HOST_ID,
            tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
        )
        .await
    });
    harness.wait_socket_removed().await;
    assert!(!broker_instance_available(&harness.paths).unwrap());

    tokio::time::advance(super::super::constants::MANAGEMENT_DRAIN_TIMEOUT).await;
    drop(_clock);
    retirement.await.unwrap().unwrap();
    harness.wait_stopped().await;

    let mut probe = [0_u8; CLOSED_STREAM_PROBE_BYTES];
    assert_eq!(silent.read(&mut probe).await.unwrap(), 0);
}
