use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Duration, Instant};
use yolo_capnp_rpc::{Confirmation, Error, Future, Method, Protocol, Reply, Service, Value};
use yolo_runtime_rpc::RuntimeConnection;
use yolo_wire::{ProtocolError, RequestEnvelope, ResponseEnvelope};
#[path = "../test_support/boundary.rs"]
mod boundary;
mod correlated;
mod subscription;

const DEADLINE: Duration = Duration::from_secs(3);
const PIPE_BYTES: usize = 65_536;
const REQUEST_ID: &str = "unknown-confirmation";
const METHOD: &str = "test.commit";

struct Committed(Arc<AtomicUsize>);
impl Confirmation for Committed {
    fn confirm(self: Box<Self>) -> Future<'static, ()> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(Error::OutcomeUnknown)
        })
    }
}
struct CommitService(Arc<AtomicUsize>);
impl Service for CommitService {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            let response =
                ResponseEnvelope::success(&request, &true).map_err(|_| Error::Invalid)?;
            Ok(Reply {
                value: yolo_capnp_codec::to_value(&response)?,
                events: None,
                on_observed: Some(Box::new(Committed(self.0.clone()))),
            })
        })
    }
}

#[tokio::test]
async fn uncertain_commit_is_typed_unknown_without_replaying_either_runtime_call_api() {
    for correlated in [true, false] {
        let (client, server) = tokio::io::duplex(PIPE_BYTES);
        let commits = Arc::new(AtomicUsize::new(0));
        let serving = tokio::spawn(yolo_capnp_rpc::serve(
            server,
            Protocol::Endpoint,
            Arc::new(CommitService(commits.clone())),
        ));
        let mut client = RuntimeConnection::open(client, Instant::now() + DEADLINE)
            .await
            .unwrap();
        let request = RequestEnvelope::new(REQUEST_ID, METHOD, Value::Null);
        let result: Result<ResponseEnvelope, _> = if correlated {
            client.exchange(&request, Instant::now() + DEADLINE).await
        } else {
            client.call(&request, Instant::now() + DEADLINE).await
        };
        assert!(matches!(result, Err(ProtocolError::OutcomeUnknown)));
        assert_eq!(commits.load(Ordering::SeqCst), 1);
        client.close();
        drop(client);
        let _ = tokio::time::timeout(DEADLINE, serving).await.unwrap();
    }
}
