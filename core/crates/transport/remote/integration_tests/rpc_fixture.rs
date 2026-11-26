use crate::protocol::{EventEnvelope, RequestEnvelope, ResponseEnvelope};
use std::sync::Arc;
use yolo_capnp_rpc::{Error, EventSource, Future, Method, Protocol, Reply, Service, Value};

pub(super) fn serve(
    stream: tokio::io::DuplexStream,
    handler: impl Fn(RequestEnvelope) -> Future<'static, Reply> + Send + Sync + 'static,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { let _ = yolo_capnp_rpc::serve(stream, Protocol::Endpoint, Arc::new(Fixture(handler))).await; })
}
struct Fixture<F>(F);
impl<F: Fn(RequestEnvelope) -> Future<'static, Reply> + Send + Sync + 'static> Service for Fixture<F> {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move { (self.0)(yolo_capnp_codec::from_value(value)?).await })
    }
}
pub(super) fn reply(response: ResponseEnvelope) -> Result<Reply, Error> {
    Ok(Reply::value(yolo_capnp_codec::to_value(&response)?))
}
pub(super) fn event_reply(response: ResponseEnvelope, mut event: EventEnvelope) -> Result<Reply, Error> {
    const PAYLOAD_FIELD: &str = "payload";
    // Adversarial event payloads bypass the normal serializer's duplicate-key guard.
    let payload = std::mem::replace(&mut event.payload, Value::Null);
    let mut value = yolo_capnp_codec::to_value(&event)?;
    if let Value::Map(fields) = &mut value {
        fields.iter_mut().find(|(key, _)| key.as_str() == Some(PAYLOAD_FIELD)).ok_or(Error::Invalid)?.1 = payload;
    }
    Ok(Reply { value: yolo_capnp_codec::to_value(&response)?, events: Some(Box::new(Event(Some(value)))), on_observed: None })
}
struct Event(Option<Value>);
impl EventSource for Event {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async { self.0.take().ok_or(Error::Closed) })
    }
}
