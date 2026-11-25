use super::support::*;
use std::sync::atomic::Ordering;
use tokio::time::timeout_at;
use yolo_hub_client::Client;
use yolo_hub_public_client_server::MAX_ACTIVE_CHANNELS;

#[tokio::test]
async fn global_channel_capacity_is_enforced_across_ssh_connections_and_recovers_after_release() {
    assert!(
        MAX_ACTIVE_CHANNELS / CONNECTION_PARTITIONS
            < yolo_hub_client_transport::MAX_CLIENT_CHANNELS
    );
    let fixture = Fixture::start().await;
    let hub = fixture.state.hub_id();
    let first = fixture.connection().await;
    let second = fixture.connection().await;
    let mut occupied = Vec::new();
    for index in COUNT_ZERO..MAX_ACTIVE_CHANNELS {
        let connection = if index % CONNECTION_PARTITIONS == COUNT_ZERO {
            &first
        } else {
            &second
        };
        // Completing Hello proves the global permit is retained by serve_admitted.
        occupied.push(probes::client(connection, hub).await);
    }
    let before = fixture.backend.calls.load(Ordering::SeqCst);
    let overflow = second.open_channel(deadline()).await.unwrap();
    let error = Client::open(overflow, hub, deadline())
        .await
        .err()
        .expect(EXPECTED_CAPACITY_REJECTION);
    probes::disconnected(error);
    assert_eq!(fixture.backend.calls.load(Ordering::SeqCst), before);
    probes::healthy(occupied.first_mut().unwrap(), hub).await;
    probes::healthy(occupied.last_mut().unwrap(), hub).await;
    let mut released = occupied.pop().unwrap();
    released.close(deadline()).await.unwrap();
    drop(released);
    let recovered = timeout_at(deadline(), async {
        loop {
            let stream = second.open_channel(deadline()).await.unwrap();
            match Client::open(stream, hub, deadline()).await {
                Ok(client) => break client,
                Err(error) => {
                    probes::disconnected(error);
                    tokio::time::sleep(CAPACITY_POLL).await;
                }
            }
        }
    })
    .await
    .expect(CAPACITY_MUST_RECOVER);
    occupied.push(recovered);
    probes::healthy(occupied.last_mut().unwrap(), hub).await;
    first.close(deadline()).await.unwrap();
    second.close(deadline()).await.unwrap();
    fixture.finish().await;
}
