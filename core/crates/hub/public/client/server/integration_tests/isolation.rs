use super::support::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::io::AsyncWriteExt;
use yolo_hub_jsonrpc_transport::FRAME_DELIMITER;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn malformed_authenticated_channel_is_retired_while_siblings_and_other_clients_keep_working()
{
    let fixture = Fixture::start().await;
    let hub = fixture.state.hub_id();
    let first = fixture.connection().await;
    let second = fixture.connection().await;
    let mut sibling = probes::client(&first, hub).await;
    let mut independent = probes::client(&second, hub).await;
    let mut broken = first.open_channel(deadline()).await.unwrap();
    let before = fixture.backend.calls.load(Ordering::SeqCst);
    broken
        .write_all(&[FRAME_DELIMITER])
        .await
        .unwrap();
    probes::closed(&mut broken).await;
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), before);
    probes::healthy(&mut sibling, hub).await;
    probes::healthy(&mut independent, hub).await;
    let mut fresh = probes::client(&first, hub).await;
    probes::healthy(&mut fresh, hub).await;
    first.close(deadline()).await.unwrap();
    second.close(deadline()).await.unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn dispatcher_panic_retires_one_channel_and_reclaims_its_global_capacity() {
    let prepared = Prepared::new();
    let backend = Arc::new(FaultyBackend {
        inner: prepared.backend.clone(),
        panic_once: AtomicBool::new(true),
    });
    let fixture = prepared.start(backend.clone()).await;
    let hub = fixture.state.hub_id();
    let first = fixture.connection().await;
    let second = fixture.connection().await;
    let mut sibling = probes::client(&first, hub).await;
    let mut independent = probes::client(&second, hub).await;
    let mut broken = probes::client(&first, hub).await;
    let error = broken
        .call(
            hub,
            Command::List {
                include_archived: true,
                after: None,
            },
            deadline(),
        )
        .await
        .err()
        .expect(EXPECTED_TRANSPORT_CLOSURE);
    probes::disconnected(error);
    assert!(!backend.panic_once.load(Ordering::SeqCst));
    probes::healthy(&mut sibling, hub).await;
    probes::healthy(&mut independent, hub).await;
    let mut recovered = Vec::new();
    for index in ACTIVE_HEALTHY_CHANNELS..yolo_hub_public_client_server::MAX_ACTIVE_CHANNELS {
        let connection = if index % CONNECTION_PARTITIONS == COUNT_ZERO {
            &first
        } else {
            &second
        };
        recovered.push(probes::client(connection, hub).await);
    }
    assert!(matches!(
        sibling
            .call(
                hub,
                Command::List {
                    include_archived: true,
                    after: None
                },
                deadline()
            )
            .await
            .unwrap(),
        Reply::Terminals { .. }
    ));
    first.close(deadline()).await.unwrap();
    second.close(deadline()).await.unwrap();
    fixture.finish().await;
}
