use crate::support::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout_at,
};

#[tokio::test]
async fn either_authenticated_endpoint_can_open_independent_channels() {
    let ((client, mut client_incoming), (server, mut server_incoming)) = pair().await;
    let mut outgoing = client.open_channel(deadline()).await.unwrap();
    let mut server_reader = timeout_at(deadline(), server_incoming.recv())
        .await
        .unwrap()
        .unwrap();
    let mut reverse = server.open_channel(deadline()).await.unwrap();
    let mut client_reader = timeout_at(deadline(), client_incoming.recv())
        .await
        .unwrap()
        .unwrap();
    outgoing.write_all(PAYLOAD).await.unwrap();
    reverse.write_all(PAYLOAD).await.unwrap();
    let mut observed = vec![u8::default(); PAYLOAD.len()];
    timeout_at(deadline(), server_reader.read_exact(&mut observed))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed, PAYLOAD);
    timeout_at(deadline(), client_reader.read_exact(&mut observed))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed, PAYLOAD);
    server_reader.write_all(PAYLOAD).await.unwrap();
    timeout_at(deadline(), outgoing.read_exact(&mut observed))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed, PAYLOAD);
    drop((outgoing, reverse, server_reader, client_reader));
    client.close(deadline()).await.unwrap();
    server.close(deadline()).await.unwrap();
    assert!(client.is_closed());
    assert!(client.open_channel(deadline()).await.is_err());
    assert!(server.open_channel(deadline()).await.is_err());
}

#[tokio::test]
async fn dropping_connection_retires_channels_and_notifies_the_other_endpoint() {
    let ((client, _), (server, mut incoming)) = pair().await;
    let outgoing = client.open_channel(deadline()).await.unwrap();
    let mut receiver = timeout_at(deadline(), incoming.recv())
        .await
        .unwrap()
        .unwrap();
    drop(client);
    let result = timeout_at(deadline(), receiver.read_u8()).await.unwrap();
    assert!(result.is_err());
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
