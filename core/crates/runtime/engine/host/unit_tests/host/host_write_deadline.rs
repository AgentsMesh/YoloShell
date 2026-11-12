use crate::host::tests::*;
use super::*;

const HOST_RECEIPT_POLL: Duration = Duration::from_millis(10);

#[tokio::test]
async fn an_unconfirmed_rpc_receipt_cannot_pin_host_response_drain_forever() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let (client, server) = UnixStream::pair().unwrap();
    let serving = tokio::spawn(Arc::clone(&host).handle_client(server));
    let client = open_host_rpc(client).await;
    let request = request(method::HOST_STATUS, &());
    let reply = client
        .request_deferred(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&request).unwrap(),
            tokio::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT,
        )
        .await
        .unwrap();
    assert!(reply.receipt.is_some());
    assert!(
        !host
            .request_drain
            .wait_for_response_drain(Duration::ZERO)
            .await
    );
    tokio::time::timeout(
        yolo_capnp_rpc::MAX_RECEIPT_WAIT + HOST_BLOCKING_TEST_TIMEOUT,
        async {
            while !host
                .request_drain
                .wait_for_response_drain(Duration::ZERO)
                .await
            {
                tokio::time::sleep(HOST_RECEIPT_POLL).await;
            }
        },
    )
    .await
    .unwrap();
    assert!(host_rpc(&client, &request).await.ok);
    drop(reply);
    drop(client);
    let _ = serving.await.unwrap();
}

#[tokio::test]
async fn dropping_an_idle_shutdown_receipt_reopens_admission_without_committing_shutdown() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let (client, server) = UnixStream::pair().unwrap();
    let serving = tokio::spawn(Arc::clone(&host).handle_client(server));
    let client = open_host_rpc(client).await;
    let request = request(method::SHUTDOWN_IF_IDLE, &());
    let reply = client
        .request_deferred(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&request).unwrap(),
            tokio::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT,
        )
        .await
        .unwrap();
    let response: ResponseEnvelope = yolo_capnp_codec::from_value(reply.value.clone()).unwrap();
    assert!(response.ok && host.request_drain.is_draining());
    drop(reply);
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async {
        while host.request_drain.is_draining() {
            tokio::time::sleep(HOST_RECEIPT_POLL).await;
        }
    })
    .await
    .unwrap();
    assert!(host.request_drain.admit_mutation().await.is_ok());
    drop(client);
    let _ = serving.await.unwrap();
}
