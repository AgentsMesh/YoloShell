use super::*;
use tokio::sync::Notify;

struct Ticket {
    published: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}
impl Ticket {
    fn publish(self) {
        self.published.fetch_add(1, Ordering::SeqCst);
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
struct Receipts {
    published: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}
impl Service for Receipts {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let ticket = Ticket {
            published: self.published.clone(),
            dropped: self.dropped.clone(),
        };
        Box::pin(async move {
            Ok(Reply {
                value,
                events: None,
                on_observed: Some(Box::new(move || ticket.publish())),
            })
        })
    }
}
#[tokio::test]
async fn observed_response_confirms_exactly_once_and_releases_the_publication_guard() {
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
    let reply = client
        .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert_eq!(reply.value, Value::Boolean(true));
    assert_eq!(published.load(Ordering::SeqCst), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
    assert_eq!(published.load(Ordering::SeqCst), 1);
}

struct Pending {
    entered: Arc<Notify>,
    dropped: Arc<AtomicUsize>,
}
impl Service for Pending {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        let ticket = Ticket {
            published: Arc::new(AtomicUsize::new(0)),
            dropped: self.dropped.clone(),
        };
        Box::pin(async move {
            self.entered.notify_one();
            std::future::pending::<()>().await;
            Ok(Reply {
                value,
                events: None,
                on_observed: Some(Box::new(move || ticket.publish())),
            })
        })
    }
}
#[tokio::test]
async fn cancelling_an_unobserved_response_releases_its_guard_without_replaying() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let entered = Arc::new(Notify::new());
    let dropped = Arc::new(AtomicUsize::new(0));
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Pending {
            entered: entered.clone(),
            dropped: dropped.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let cloned = client.clone();
    let request = tokio::spawn(async move {
        cloned
            .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
            .await
    });
    tokio::time::timeout(TIMEOUT, entered.notified())
        .await
        .unwrap();
    request.abort();
    let _ = request.await;
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn aborting_serve_cancels_domain_work_even_while_the_client_still_holds_the_call() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let entered = Arc::new(Notify::new());
    let dropped = Arc::new(AtomicUsize::new(0));
    let serving = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Pending {
            entered: entered.clone(),
            dropped: dropped.clone(),
        }),
    ));
    let client = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let active = client.clone();
    let request = tokio::spawn(async move {
        active
            .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
            .await
    });
    tokio::time::timeout(TIMEOUT, entered.notified())
        .await
        .unwrap();
    serving.abort();
    let _ = serving.await;
    tokio::time::timeout(TIMEOUT, async {
        while dropped.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert!(request.await.unwrap().is_err());
    drop(client);
}

#[path = "receipt_expiry.rs"]
mod expiry;
