use super::super::constants::MANAGEMENT_DRAIN_TIMEOUT;
use super::server_support::Harness;
use super::support::{CONFIG_REVISION, ConnectStep, FakeFactory, test_config};
use crate::remote::{
    BROKER_COMMAND_CANCELLATION_PROGRAM, BrokerCommandCleanupFixture, CommandPolicy,
};
use crate::remote_broker::{BrokerClient, BrokerCommandRequest};

const OBSERVATION_ATTEMPTS: usize = 1_000;
const OBSERVATION_FAILURE: &str = "remote command state did not converge";
const PRIMARY_CONNECTION_INDEX: usize = 0;

#[tokio::test(start_paused = true)]
async fn rotation_management_drain_kills_and_closes_remote_command_channel() {
    let fixture = BrokerCommandCleanupFixture::connect().await;
    let factory = FakeFactory::new(
        vec![fixture.connection()],
        vec![ConnectStep::Connection(PRIMARY_CONNECTION_INDEX)],
    );
    let mut config = test_config();
    config.command_policy =
        CommandPolicy::new([BROKER_COMMAND_CANCELLATION_PROGRAM.into()]).unwrap();
    let harness = Harness::start_with_config(factory, config).await;
    let client = BrokerClient::new(harness.paths.management_socket());
    let command = tokio::spawn(async move {
        client
            .run_command(BrokerCommandRequest::standard(
                BROKER_COMMAND_CANCELLATION_PROGRAM,
                Vec::new(),
            ))
            .await
    });

    wait_until(|| fixture.command_is_live()).await;
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    tokio::task::yield_now().await;
    tokio::time::advance(MANAGEMENT_DRAIN_TIMEOUT).await;

    wait_until(|| fixture.command_cleanup_is_exact()).await;
    assert!(command.await.unwrap().is_err());
    harness.wait_stopped().await;
}

async fn wait_until(condition: impl Fn() -> bool) {
    for _ in 0..OBSERVATION_ATTEMPTS {
        if condition() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("{OBSERVATION_FAILURE}");
}
