use super::*;

#[tokio::test]
async fn oversized_call_is_rejected_locally_without_dispatch_or_poisoning_connection() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let calls = Arc::new(AtomicUsize::new(0));
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Echo(calls.clone())),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let too_large = Value::Data(vec![u8::default(); yolo_capnp_codec::MAX_MESSAGE_BYTES]);
    let rejected = client
        .request(Method::Call, too_large, Instant::now() + TIMEOUT)
        .await;
    assert!(matches!(rejected, Err(Error::Invalid)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        client
            .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
            .await
            .unwrap()
            .value,
        Value::Boolean(true)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(client);
    tokio::time::timeout(TIMEOUT, serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
