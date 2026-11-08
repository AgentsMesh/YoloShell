use super::*;
use tokio::sync::Notify;

const RESPONSE_DEADLINE: Duration = Duration::from_millis(100);
const EXPIRED_OFFSET: Duration = Duration::from_millis(1);

#[tokio::test]
async fn expired_preflight_never_enters_the_service_or_poisons_following_calls() {
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
    let result = client
        .request(
            Method::Call,
            Value::Boolean(true),
            Instant::now() - EXPIRED_OFFSET,
        )
        .await;
    assert!(matches!(result, Err(Error::Deadline)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    client
        .request(Method::Call, Value::Boolean(true), Instant::now() + TIMEOUT)
        .await
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
}

struct CommittedWithoutReply {
    entered: Arc<Notify>,
    commits: Arc<AtomicUsize>,
}
impl Service for CommittedWithoutReply {
    fn call(&self, _: Method, _: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            self.commits.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn timeout_or_disconnect_after_dispatch_is_unknown_and_never_replayed() {
    for (protocol, method) in [
        (Protocol::Endpoint, Method::Call),
        (Protocol::Peer, Method::Execute),
    ] {
        for disconnect in [false, true] {
            let (client, server) = tokio::io::duplex(PIPE_BYTES);
            let entered = Arc::new(Notify::new());
            let commits = Arc::new(AtomicUsize::new(0));
            let serving = tokio::spawn(yolo_capnp_rpc::serve(
                server,
                protocol,
                Arc::new(CommittedWithoutReply {
                    entered: entered.clone(),
                    commits: commits.clone(),
                }),
            ));
            let client = Connection::open(client, protocol, Instant::now() + TIMEOUT)
                .await
                .unwrap();
            let issuing = client.clone();
            let operation = tokio::spawn(async move {
                issuing
                    .request(method, Value::Null, Instant::now() + RESPONSE_DEADLINE)
                    .await
            });
            tokio::time::timeout(TIMEOUT, entered.notified())
                .await
                .unwrap();
            if disconnect {
                serving.abort();
            }
            let result = tokio::time::timeout(TIMEOUT, operation)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(result, Err(yolo_capnp_rpc::Error::OutcomeUnknown)));
            assert_eq!(commits.load(Ordering::SeqCst), 1);
            drop(client);
            let _ = tokio::time::timeout(TIMEOUT, serving).await.unwrap();
        }
    }
}
