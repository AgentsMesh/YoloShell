use super::*;
use tokio::sync::Notify;

struct Shutdown(Arc<Notify>);
impl Service for Shutdown {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let stop = self.0.clone();
        Box::pin(async move {
            Ok(Reply {
                value,
                events: None,
                on_observed: Some(Box::new(move || stop.notify_one())),
            })
        })
    }
}

#[tokio::test]
async fn confirmation_triggered_shutdown_flushes_the_positive_receipt_before_disconnect() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let stop = Arc::new(Notify::new());
    let shutdown = stop.clone();
    let serving = tokio::spawn(async move {
        tokio::select! {
            result = yolo_capnp_rpc::serve(server, Protocol::Endpoint, Arc::new(Shutdown(shutdown))) => { result.unwrap(); },
            _ = stop.notified() => {},
        }
    });
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let result = client
        .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert_eq!(result.value, Value::Boolean(true));
    serving.await.unwrap();
}

#[tokio::test]
async fn cancelling_server_future_releases_idle_service_ownership_immediately() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let service = Arc::new(Echo(Arc::new(AtomicUsize::new(0))));
    let weak = Arc::downgrade(&service);
    let serving = tokio::spawn(yolo_capnp_rpc::serve(server, Protocol::Endpoint, service));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    client
        .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    serving.abort();
    let _ = serving.await;
    assert!(weak.upgrade().is_none());
    drop(client);
}
