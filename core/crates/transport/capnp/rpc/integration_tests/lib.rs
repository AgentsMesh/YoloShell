use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Duration, Instant};
use yolo_capnp_rpc::{
    Connection, Error, EventSource, Future, Method, Protocol, Reply, Service, Value,
};
mod budgets;
mod events;
mod event_closure;
mod isolation;
mod proxy;
mod receipt_loss;
mod receipts;
mod shutdown;
mod uncertain_dispatch;

const TIMEOUT: Duration = Duration::from_secs(3);
const CALL_TIMEOUT: Duration = Duration::from_millis(100);
const PIPE_BYTES: usize = 65_536;
const PAYLOAD: &str = "native capability RPC";
struct Echo(Arc<AtomicUsize>);
impl Service for Echo {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if value == Value::Null {
                std::future::pending::<()>().await;
            }
            let events =
                (method == Method::Execute).then(|| Box::new(Events) as Box<dyn EventSource>);
            Ok(Reply {
                value,
                events,
                on_observed: None,
            })
        })
    }
}
struct Events;
impl EventSource for Events {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async { Ok(Value::Text(PAYLOAD.into())) })
    }
}

#[tokio::test]
async fn peer_methods_and_returned_event_capability_share_one_real_rpc_connection() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let calls = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Peer,
        Arc::new(Echo(calls.clone())),
    ));
    let connection = Connection::open(client, Protocol::Peer, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    for method in [Method::Discover, Method::Execute] {
        let reply = connection
            .request(
                method,
                Value::Text(PAYLOAD.into()),
                Instant::now() + TIMEOUT,
            )
            .await
            .unwrap();
        assert_eq!(reply.value, Value::Text(PAYLOAD.into()));
        if let Some(mut events) = reply.events {
            assert_eq!(
                events.next(Instant::now() + TIMEOUT).await.unwrap(),
                Value::Text(PAYLOAD.into())
            );
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    drop(connection);
    tokio::time::timeout(TIMEOUT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn timed_out_call_is_not_replayed_and_does_not_block_another_call() {
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let calls = Arc::new(AtomicUsize::new(0));
    let server = tokio::spawn(yolo_capnp_rpc::serve(
        server,
        Protocol::Endpoint,
        Arc::new(Echo(calls.clone())),
    ));
    let connection = Connection::open(client, Protocol::Endpoint, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let first = connection.request(Method::Call, Value::Null, Instant::now() + CALL_TIMEOUT);
    let second = connection.request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT);
    let (first, second) = tokio::join!(first, second);
    assert!(matches!(first, Err(Error::OutcomeUnknown)));
    assert_eq!(second.unwrap().value, Value::Boolean(true));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    connection.close();
    drop(connection);
    let _ = tokio::time::timeout(TIMEOUT, server).await.unwrap();
}
