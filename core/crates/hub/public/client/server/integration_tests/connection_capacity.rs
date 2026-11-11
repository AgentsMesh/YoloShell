use super::support::*;
use tokio::{net::UnixStream, time::timeout_at};
use yolo_hub_public_client_server::MAX_CONNECTIONS;

#[tokio::test]
async fn pending_ssh_handshakes_cannot_exceed_server_connection_limit_or_destroy_healthy_clients() {
    let fixture = Fixture::start().await;
    let hub = fixture.state.hub_id();
    let connection = fixture.connection().await;
    let mut healthy_client = probes::client(&connection, hub).await;
    let mut stalled = Vec::new();
    for _ in COUNT_ONE..MAX_CONNECTIONS {
        stalled.push(fixture.pending_handshake().await);
    }
    let mut overflow = timeout_at(deadline(), UnixStream::connect(&fixture.socket))
        .await
        .unwrap()
        .unwrap();
    probes::closed(&mut overflow).await;
    probes::healthy(&mut healthy_client, hub).await;
    drop(stalled.pop().unwrap());
    let recovered = fixture.eventually_connection().await;
    let mut fresh = probes::client(&recovered, hub).await;
    probes::healthy(&mut fresh, hub).await;
    probes::healthy(&mut healthy_client, hub).await;
    drop(stalled);
    recovered.close(deadline()).await.unwrap();
    connection.close(deadline()).await.unwrap();
    fixture.finish().await;
}
