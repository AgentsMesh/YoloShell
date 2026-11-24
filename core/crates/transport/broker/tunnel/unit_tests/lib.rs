use crate::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_local_unix::LocalUnixStream;

const PAYLOAD: &[u8] = b"independent tunnel";
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(2);
mod shutdown;

#[tokio::test]
async fn tunnel_is_bidirectional_and_close_wakes_pending_io_without_closing_sibling() {
    let (local, mut remote) = LocalUnixStream::pair().unwrap();
    let (other, mut sibling) = LocalUnixStream::pair().unwrap();
    let opened = yolo_broker_hub_test_support::opened();
    let mut tunnel = BrokerHubTunnel::new(local, opened.clone()).unwrap();
    let mut other = BrokerHubTunnel::new(other, opened.clone()).unwrap();
    assert_eq!(tunnel.opened(), &opened);
    assert_eq!(tunnel.binding(), &opened.request.binding);
    tunnel.write_all(PAYLOAD).await.unwrap();
    tunnel.flush().await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    remote.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
    remote.write_all(PAYLOAD).await.unwrap();
    tunnel.read_exact(&mut bytes).await.unwrap();
    let close = tunnel.close_handle();
    let (entered, pending) = tokio::sync::oneshot::channel();
    let blocked = tokio::spawn(async move {
        let mut byte = u8::default();
        let reading = tunnel.read(std::slice::from_mut(&mut byte));
        tokio::pin!(reading);
        let mut entered = Some(entered);
        std::future::poll_fn(|context| {
            use std::future::Future;
            let result = reading.as_mut().poll(context);
            if result.is_pending() {
                if let Some(entered) = entered.take() {
                    let _ = entered.send(());
                }
            }
            result
        })
        .await
    });
    tokio::time::timeout(DEADLINE, pending)
        .await
        .unwrap()
        .unwrap();
    close.close();
    close.close();
    assert_eq!(
        tokio::time::timeout(DEADLINE, blocked)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotConnected
    );
    let mut byte = u8::default();
    assert_eq!(
        remote.read(std::slice::from_mut(&mut byte)).await.unwrap(),
        usize::default()
    );
    other.write_all(PAYLOAD).await.unwrap();
    sibling.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
}

#[tokio::test]
async fn retained_close_handle_after_drop_cannot_close_a_reused_descriptor() {
    let (local, mut remote) = LocalUnixStream::pair().unwrap();
    let tunnel = BrokerHubTunnel::new(local, yolo_broker_hub_test_support::opened()).unwrap();
    let close = tunnel.close_handle();
    drop(tunnel);
    let mut byte = u8::default();
    assert_eq!(
        remote.read(std::slice::from_mut(&mut byte)).await.unwrap(),
        usize::default()
    );
    let (mut next, mut peer) = LocalUnixStream::pair().unwrap();
    close.close();
    next.write_all(PAYLOAD).await.unwrap();
    let mut bytes = vec![u8::default(); PAYLOAD.len()];
    peer.read_exact(&mut bytes).await.unwrap();
    assert_eq!(bytes, PAYLOAD);
}

#[tokio::test]
async fn explicit_close_fences_write_flush_and_shutdown() {
    let (local, _remote) = LocalUnixStream::pair().unwrap();
    let mut tunnel = BrokerHubTunnel::new(local, yolo_broker_hub_test_support::opened()).unwrap();
    tunnel.close();
    assert!(tunnel.write_all(PAYLOAD).await.is_err());
    assert!(tunnel.flush().await.is_err());
    tunnel.shutdown().await.unwrap();
}
