use super::*;
use tokio::sync::Notify;

struct Streams {
    dropped: Arc<AtomicUsize>,
    entered: Arc<Notify>,
}
impl Service for Streams {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let source = Stream {
            dropped: self.dropped.clone(),
            entered: self.entered.clone(),
        };
        Box::pin(async move {
            Ok(Reply {
                value,
                events: Some(Box::new(source)),
                on_observed: None,
            })
        })
    }
}
struct Stream {
    dropped: Arc<AtomicUsize>,
    entered: Arc<Notify>,
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
impl EventSource for Stream {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async {
            self.entered.notify_one();
            std::future::pending().await
        })
    }
}
#[tokio::test]
async fn dropping_a_pending_next_detaches_source_without_closing_other_capabilities() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let dropped = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Peer,
        Arc::new(Streams {
            dropped: dropped.clone(),
            entered: entered.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Peer, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let mut events = client
        .request(Method::Execute, Value::Null, Instant::now() + TIMEOUT)
        .await
        .unwrap()
        .events
        .unwrap();
    let pending = tokio::spawn(async move { events.next_unbounded().await });
    tokio::time::timeout(TIMEOUT, entered.notified())
        .await
        .unwrap();
    pending.abort();
    let _ = pending.await;
    tokio::time::timeout(TIMEOUT, async {
        while dropped.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    let reply = client
        .request(
            Method::Discover,
            Value::Boolean(true),
            Instant::now() + TIMEOUT,
        )
        .await
        .unwrap();
    assert_eq!(reply.value, Value::Boolean(true));
    drop(reply);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
}
