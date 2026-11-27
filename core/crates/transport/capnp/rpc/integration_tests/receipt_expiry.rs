use super::*;

#[tokio::test]
async fn retained_unconfirmed_capability_expires_its_guard_without_committing() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let published = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Receipts {
            published: published.clone(),
            dropped: dropped.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let retained = client
        .request_deferred(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert!(retained.receipt.is_some());
    assert_eq!(published.load(Ordering::SeqCst), 0);
    tokio::time::timeout(yolo_capnp_rpc::MAX_RECEIPT_WAIT + TIMEOUT, async {
        while dropped.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(published.load(Ordering::SeqCst), 0);
    assert!(matches!(
        retained
            .receipt
            .unwrap()
            .confirm(Instant::now() + TIMEOUT)
            .await,
        Err(Error::OutcomeUnknown)
    ));
    let another = client
        .request(
            Method::Call,
            Value::Boolean(false),
            Instant::now() + TIMEOUT,
        )
        .await
        .unwrap();
    assert_eq!(another.value, Value::Boolean(false));
    assert_eq!(published.load(Ordering::SeqCst), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
}
