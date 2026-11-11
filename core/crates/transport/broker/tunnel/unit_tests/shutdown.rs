use super::*;

#[tokio::test]
async fn live_write_shutdown_delivers_eof_but_preserves_reading_until_explicit_close() {
    let (local, mut remote) = LocalUnixStream::pair().unwrap();
    let mut tunnel = BrokerHubTunnel::new(local, yolo_broker_hub_test_support::opened()).unwrap();
    tunnel.write_all(PAYLOAD).await.unwrap();
    tunnel.flush().await.unwrap();
    tunnel.shutdown().await.unwrap();
    let mut received = Vec::new();
    tokio::time::timeout(DEADLINE, remote.read_to_end(&mut received))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received, PAYLOAD);
    remote.write_all(PAYLOAD).await.unwrap();
    let mut reply = vec![u8::default(); PAYLOAD.len()];
    tokio::time::timeout(DEADLINE, tunnel.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reply, PAYLOAD);
    tunnel.close();
    assert_eq!(
        tunnel.read(&mut reply).await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    assert_eq!(
        tunnel.write(PAYLOAD).await.unwrap_err().kind(),
        std::io::ErrorKind::NotConnected
    );
    tunnel.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_acknowledgement_is_rejected_and_releases_its_socket_before_io() {
    let (local, mut remote) = LocalUnixStream::pair().unwrap();
    let mut invalid = yolo_broker_hub_test_support::opened();
    invalid.request.binding.connection_generation = u64::default();
    let error = BrokerHubTunnel::new(local, invalid).err().unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    let mut byte = u8::default();
    assert_eq!(
        tokio::time::timeout(DEADLINE, remote.read(std::slice::from_mut(&mut byte)))
            .await
            .unwrap()
            .unwrap(),
        usize::default()
    );
}
