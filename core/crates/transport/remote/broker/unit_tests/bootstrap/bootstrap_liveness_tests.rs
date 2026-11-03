use super::super::*;
use super::support::*;
use crate::remote::RemoteError;
use std::sync::Arc;

const FIRST_CONNECTION_GENERATION: u64 = 1;
const SECOND_CONNECTION_GENERATION: u64 = FIRST_CONNECTION_GENERATION + 1;
const FIRST_CONNECTION_INDEX: usize = 0;
const SECOND_CONNECTION_INDEX: usize = FIRST_CONNECTION_INDEX + 1;
const EXPECTED_BOOTSTRAP_CONNECTIONS: usize = 1;
const EXPECTED_BOOTSTRAP_PROBES: usize = 1;
const EXPECTED_REPLACEMENT_CONNECTIONS: usize = 2;

fn pool(factory: Arc<FakeFactory>) -> BrokerConnectionPool {
    let config = test_config();
    BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    )
}

#[tokio::test]
async fn bootstrap_probes_a_real_runtime_control_channel() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(FIRST_CONNECTION_INDEX)],
    );
    let pool = pool(Arc::clone(&factory));

    let status = pool.bootstrap().await.unwrap();

    assert_eq!(status.connection_id, Some(connection.id()));
    assert_eq!(status.connection_generation, FIRST_CONNECTION_GENERATION);
    assert!(status.supports(CONNECTION_BOOTSTRAP_CAPABILITY));
    assert_eq!(factory.calls(), EXPECTED_BOOTSTRAP_CONNECTIONS);
    assert_eq!(
        connection.opens(),
        vec![connection.id(); EXPECTED_BOOTSTRAP_PROBES]
    );
    assert_eq!(connection.commands(), usize::default());
    assert_eq!(connection.files(), usize::default());
}

#[tokio::test]
async fn bootstrap_replaces_a_dead_cached_connection_and_advances_generation() {
    assert_cached_replacement(OpenStep::TransportFailure).await;
}

#[tokio::test]
async fn bootstrap_replaces_cached_connection_when_runtime_forward_eofs_before_hello() {
    assert_cached_replacement(OpenStep::ImmediateEof).await;
}

async fn assert_cached_replacement(failure: OpenStep) {
    let first = FakeConnection::new([OpenStep::Channel, failure]);
    let second = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&first), Arc::clone(&second)],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(SECOND_CONNECTION_INDEX),
        ],
    );
    let pool = pool(Arc::clone(&factory));
    let initial = pool.bootstrap().await.unwrap();

    let replacement = pool.bootstrap().await.unwrap();

    assert_eq!(initial.connection_id, Some(first.id()));
    assert_eq!(initial.connection_generation, FIRST_CONNECTION_GENERATION);
    assert_eq!(replacement.connection_id, Some(second.id()));
    assert_eq!(
        replacement.connection_generation,
        SECOND_CONNECTION_GENERATION
    );
    assert_eq!(factory.calls(), EXPECTED_REPLACEMENT_CONNECTIONS);
}

#[tokio::test]
async fn bootstrap_non_transport_probe_failure_does_not_reconnect() {
    let first = FakeConnection::new([OpenStep::Rejected]);
    let replacement = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![first, replacement],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(SECOND_CONNECTION_INDEX),
        ],
    );

    let error = pool(Arc::clone(&factory)).bootstrap().await.unwrap_err();

    assert!(matches!(error, RemoteError::ChannelRejected));
    assert_eq!(factory.calls(), EXPECTED_BOOTSTRAP_CONNECTIONS);
}
