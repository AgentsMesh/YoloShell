use super::support::*;
use crate::protocol::{RequestEnvelope, method};
use crate::remote::RemoteCleanupRegistry;
use crate::remote_broker::socket_probe::{BrokerSocketProbe, bounded_broker_socket_probe};
use crate::remote_broker::{BrokerConnectFuture, BrokerConnectionFactory, BrokerConnectionPool};
use std::sync::Arc;

const ATTACHMENT_LIMIT: usize = 1;
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(50);
const INITIAL_GENERATION: u64 = 0;
const FIRST_CONNECTION_INDEX: usize = 0;
const SECOND_CONNECTION_INDEX: usize = 1;

struct PendingFactory;

impl BrokerConnectionFactory for PendingFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test]
async fn admitted_proxy_contains_local_eof_during_pending_remote_open() {
    let config = test_config();
    let pool = Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        Arc::new(PendingFactory),
    ));
    let attachments = Arc::new(tokio::sync::Semaphore::new(ATTACHMENT_LIMIT));
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let proxy = tokio::spawn(crate::remote_broker::runtime_proxy::proxy_admitted(server.into(), pool, attachments));
    let client = super::runtime_client::connect(client).await;
    let pending = super::runtime_client::pending(&client, request);
    tokio::task::yield_now().await;
    pending.abort(); let _ = pending.await;
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test]
async fn same_generation_stale_identity_cannot_clear_a_replacement_connection() {
    let first = FakeConnection::new([]);
    let second = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&first), Arc::clone(&second)],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(SECOND_CONNECTION_INDEX),
        ],
    );
    let config = test_config();
    let pool = BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    );
    let stale = pool.observed_connection().await.unwrap();
    pool.set_generation_for_test(INITIAL_GENERATION).await;
    let replacement = pool.observed_connection().await.unwrap();
    assert_eq!(stale.generation(), replacement.generation());
    assert_ne!(stale.id(), replacement.id());

    pool.invalidate(&stale).await;

    assert_eq!(
        pool.status().await.connection_generation,
        replacement.generation()
    );
}

#[tokio::test]
async fn socket_probe_preserves_non_refusal_io_failures() {
    let probe = bounded_broker_socket_probe(
        std::future::ready(Err::<(), _>(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        ))),
        PROBE_TIMEOUT,
    )
    .await;

    assert!(matches!(probe, BrokerSocketProbe::Indeterminate(_)));
}

#[tokio::test(start_paused = true)]
async fn socket_probe_reports_its_bounded_timeout() {
    let probe =
        bounded_broker_socket_probe(std::future::pending::<std::io::Result<()>>(), PROBE_TIMEOUT)
            .await;

    assert!(matches!(probe, BrokerSocketProbe::TimedOut));
}
