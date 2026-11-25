use super::*;

const REMOTE_DETAIL: &str = "remote observation failed";

struct Closing;
impl Service for Closing {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            if value == Value::Null {
                return Err(Error::Closed);
            }
            Ok(Reply {
                value,
                events: Some(Box::new(Closed)),
                on_observed: None,
            })
        })
    }
}
struct Closed;
impl EventSource for Closed {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async { Err(Error::Closed) })
    }
}
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

#[tokio::test]
async fn explicit_event_closure_survives_a_real_proxy_without_closing_independent_calls() {
    let (upstream, origin) = tokio::io::duplex(PIPE_BYTES);
    let origin = tokio::spawn(yolo_capnp_rpc::serve(
        origin,
        Protocol::Peer,
        Arc::new(Closing),
    ));
    let upstream = Connection::open(upstream, Protocol::Peer, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let (client, bridge) = tokio::io::duplex(PIPE_BYTES);
    let bridge = tokio::spawn(yolo_capnp_rpc::serve(
        bridge,
        Protocol::Peer,
        Arc::new(Proxy(upstream)),
    ));
    let client = Connection::open(client, Protocol::Peer, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    for bounded in [false, true] {
        let mut events = client
            .request(
                Method::Discover,
                Value::Boolean(true),
                Instant::now() + TIMEOUT,
            )
            .await
            .unwrap()
            .events
            .unwrap();
        let result = if bounded {
            events.next(Instant::now() + TIMEOUT).await
        } else {
            events.next_unbounded().await
        };
        assert_eq!(result, Err(Error::Closed));
    }
    let reply = client
        .request(
            Method::Discover,
            Value::Boolean(false),
            Instant::now() + TIMEOUT,
        )
        .await
        .unwrap();
    assert_eq!(reply.value, Value::Boolean(false));
    drop(reply);
    drop(client);
    tokio::time::timeout(TIMEOUT, bridge)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    tokio::time::timeout(TIMEOUT, origin)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn a_disconnected_exception_after_mutation_dispatch_remains_unknown() {
    for (protocol, method) in [
        (Protocol::Endpoint, Method::Call),
        (Protocol::Peer, Method::Execute),
    ] {
        for deferred in [false, true] {
            let (client, server) = tokio::io::duplex(PIPE_BYTES);
            let serving = tokio::spawn(yolo_capnp_rpc::serve(server, protocol, Arc::new(Closing)));
            let client = Connection::open(client, protocol, Instant::now() + TIMEOUT)
                .await
                .unwrap();
            let result = if deferred {
                client
                    .request_deferred(method, Value::Null, Instant::now() + TIMEOUT)
                    .await
            } else {
                client
                    .request(method, Value::Null, Instant::now() + TIMEOUT)
                    .await
            };
            assert!(matches!(result, Err(Error::OutcomeUnknown)));
            drop(client);
            tokio::time::timeout(TIMEOUT, serving)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
    }
}

#[test]
fn remote_exception_kinds_do_not_reclassify_remote_failure_as_local_capacity_or_eof() {
    assert_eq!(
        Error::from(capnp::Error::disconnected(REMOTE_DETAIL.into())),
        Error::Closed
    );
    for kind in [
        capnp::ErrorKind::Failed,
        capnp::ErrorKind::Overloaded,
        capnp::ErrorKind::Unimplemented,
    ] {
        let error = capnp::Error {
            kind,
            extra: REMOTE_DETAIL.into(),
        };
        let expected = error.to_string();
        assert_eq!(Error::from(error), Error::Remote(expected));
    }
}
