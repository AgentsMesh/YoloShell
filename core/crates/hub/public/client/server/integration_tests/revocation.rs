use super::support::*;
use yolo_hub_client::Client;
use yolo_hub_jsonrpc_transport::{Incoming, TransportError};

fn disconnected(result: Result<Incoming<yolo_hub_jsonrpc_protocol::Answer>, TransportError>) {
    assert!(
        matches!(result, Ok(Incoming::Eof) | Err(TransportError::Io(_))),
        "expected transport closure, got {result:?}"
    );
}

#[tokio::test]
async fn registry_revocation_closes_a_quiet_rpc_channel_without_a_followup_request() {
    let fixture = Fixture::start().await;
    let connection = fixture.connection().await;
    let client = Client::open(
        connection.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    let mut channel = client.into_channel().unwrap();
    fixture.registry.close();
    let result = channel
        .receive::<yolo_hub_jsonrpc_protocol::Answer>(deadline())
        .await;
    disconnected(result);
    connection.close(deadline()).await.unwrap();
    // Registry loss is an ingress service failure, separate from origin PTY custody.
    drop(fixture);
}

#[tokio::test]
async fn scope_replacement_invalidates_the_connection_but_allows_fresh_authentication() {
    let fixture = Fixture::start().await;
    let original = fixture.connection().await;
    let client = Client::open(
        original.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    let mut channel = client.into_channel().unwrap();
    let mut next = fixture.registry.current().unwrap();
    let previous = next.revision;
    next.revision += 1;
    fixture.registry.replace(previous, next).unwrap();
    disconnected(
        channel
            .receive::<yolo_hub_jsonrpc_protocol::Answer>(deadline())
            .await,
    );
    let fresh = fixture.connection().await;
    let _client = Client::open(
        fresh.open_channel(deadline()).await.unwrap(),
        fixture.state.hub_id(),
        deadline(),
    )
    .await
    .unwrap();
    fresh.close(deadline()).await.unwrap();
    original.close(deadline()).await.unwrap();
    fixture.finish().await;
}
