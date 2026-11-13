use super::*;

struct Proxy(Connection);
impl Service for Proxy {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            self.0
                .request_deferred(method, value, Instant::now() + TIMEOUT)
                .await
                .map(yolo_capnp_rpc::RemoteReply::into_service_reply)
        })
    }
}
struct Published(Arc<AtomicUsize>);
impl Service for Published {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let published = self.0.clone();
        Box::pin(async move {
            Ok(Reply {
                value,
                events: None,
                on_observed: Some(Box::new(move || {
                    published.fetch_add(1, Ordering::SeqCst);
                })),
            })
        })
    }
}

#[tokio::test]
async fn rpc_proxy_does_not_confirm_upstream_until_the_downstream_observer_confirms() {
    let (upstream, origin) = tokio::io::duplex(PIPE_BYTES);
    let published = Arc::new(AtomicUsize::new(0));
    let origin = tokio::spawn(yolo_capnp_rpc::serve(
        origin,
        Protocol::Endpoint,
        Arc::new(Published(published.clone())),
    ));
    let upstream = Connection::open(upstream, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let (client, bridge) = tokio::io::duplex(PIPE_BYTES);
    let bridge = tokio::spawn(yolo_capnp_rpc::serve(
        bridge,
        Protocol::Endpoint,
        Arc::new(Proxy(upstream)),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let reply = client
        .request_deferred(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert_eq!(reply.value, Value::Boolean(true));
    assert_eq!(published.load(Ordering::SeqCst), 0);
    reply
        .receipt
        .unwrap()
        .confirm(Instant::now() + TIMEOUT)
        .await
        .unwrap();
    tokio::time::timeout(TIMEOUT, async {
        while published.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(published.load(Ordering::SeqCst), 1);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, bridge).await.unwrap();
    let _ = tokio::time::timeout(TIMEOUT, origin).await.unwrap();
}
