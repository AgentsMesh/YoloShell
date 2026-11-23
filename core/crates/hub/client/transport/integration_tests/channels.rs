use crate::support::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};
use yolo_hub_client_transport::MAX_QUEUED_CHANNELS;

#[tokio::test]
async fn one_authenticated_connection_carries_independent_admitted_channels() {
    let (client, server, mut incoming, _registry) = pair().await;
    let mut first = client.open_channel(deadline()).await.unwrap();
    let admitted_first = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    let mut second = client.open_channel(deadline()).await.unwrap();
    let mut admitted_second = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(admitted_first.connection_id, server.connection_id());
    assert_eq!(admitted_second.connection_id, server.connection_id());
    assert_eq!(
        admitted_second.admission.grant(),
        server.admission().grant()
    );
    first.write_all(PAYLOAD).await.unwrap();
    drop((first, admitted_first));
    second.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), admitted_second.stream.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    admitted_second.stream.write_all(PAYLOAD).await.unwrap();
    timeout_at(deadline(), second.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
    assert!(client.is_closed());
    assert!(client.open_channel(deadline()).await.is_err());
}

#[tokio::test]
async fn bounded_incoming_queue_rejects_overflow_without_retiring_existing_streams() {
    let (client, server, mut incoming, _registry) = pair().await;
    let mut streams = Vec::new();
    for _ in 0..MAX_QUEUED_CHANNELS {
        streams.push(client.open_channel(deadline()).await.unwrap());
    }
    assert!(client.open_channel(deadline()).await.is_err());
    assert!(!client.is_closed());
    let mut admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    streams
        .first_mut()
        .unwrap()
        .write_all(PAYLOAD)
        .await
        .unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), admitted.stream.read_exact(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    let recovered = client.open_channel(deadline()).await.unwrap();
    drop((streams, recovered));
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
}

#[tokio::test]
async fn dropping_connection_closes_live_streams_without_dropping_their_wrappers() {
    let (client, server, mut incoming, _registry) = pair().await;
    let outgoing = client.open_channel(deadline()).await.unwrap();
    let mut admitted = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    drop(client);
    assert!(
        timeout_at(deadline(), admitted.stream.read_u8())
            .await
            .unwrap()
            .is_err()
    );
    timeout_at(deadline(), async {
        while !server.is_closed() {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    drop(outgoing);
    server.close(deadline()).await.unwrap();
}
