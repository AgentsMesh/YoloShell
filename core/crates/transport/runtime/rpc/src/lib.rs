//! Domain-correlated runtime calls over Cap'n Proto capabilities.
use serde::{Serialize, de::DeserializeOwned};
use tokio::time::Instant;
use yolo_capnp_rpc::{Connection, Duplex, Method, Protocol, RemoteEvents};
use yolo_wire::{EventEnvelope, ProtocolError, RequestEnvelope, ResponseEnvelope};

pub struct RuntimeConnection {
    connection: Connection,
    events: Option<RemoteEvents>,
}

impl RuntimeConnection {
    pub async fn forward(
        &mut self,
        request: &RequestEnvelope,
        deadline: Instant,
    ) -> Result<yolo_capnp_rpc::RemoteReply, ProtocolError> {
        let payload = yolo_capnp_codec::to_value(request).map_err(ProtocolError::Encode)?;
        let reply = self
            .connection
            .request_deferred(Method::Call, payload, deadline)
            .await
            .map_err(error)?;
        let envelope =
            yolo_capnp_codec::from_value(reply.value.clone()).map_err(ProtocolError::Decode)?;
        yolo_protocol_validation::validate_response(request, &envelope)?;
        Ok(reply)
    }
    pub async fn open(io: impl Duplex + 'static, deadline: Instant) -> Result<Self, ProtocolError> {
        Ok(Self {
            connection: Connection::open(io, Protocol::Endpoint, deadline)
                .await
                .map_err(error)?,
            events: None,
        })
    }
    pub async fn exchange(
        &mut self,
        request: &RequestEnvelope,
        deadline: Instant,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let payload = yolo_capnp_codec::to_value(request).map_err(ProtocolError::Encode)?;
        let response = self
            .connection
            .request_deferred(Method::Call, payload, deadline)
            .await
            .map_err(error)?;
        let envelope =
            yolo_capnp_codec::from_value(response.value).map_err(ProtocolError::Decode)?;
        yolo_protocol_validation::validate_response(request, &envelope)?;
        if let Some(receipt) = response.receipt {
            receipt.confirm(deadline).await.map_err(error)?;
        }
        if let Some(events) = response.events {
            self.events = Some(events);
        }
        Ok(envelope)
    }
    pub async fn call<T: Serialize, R: DeserializeOwned>(
        &mut self,
        request: &T,
        deadline: Instant,
    ) -> Result<R, ProtocolError> {
        let payload = yolo_capnp_codec::to_value(request).map_err(ProtocolError::Encode)?;
        let response = self
            .connection
            .request_deferred(Method::Call, payload, deadline)
            .await
            .map_err(error)?;
        let result = yolo_capnp_codec::from_value(response.value).map_err(ProtocolError::Decode)?;
        if let Some(receipt) = response.receipt {
            receipt.confirm(deadline).await.map_err(error)?;
        }
        if let Some(events) = response.events {
            self.events = Some(events);
        }
        Ok(result)
    }
    pub async fn next_event(&mut self, deadline: Instant) -> Result<EventEnvelope, ProtocolError> {
        let events = self
            .events
            .as_mut()
            .ok_or(ProtocolError::ConnectionPoisoned)?;
        let value = events.next(deadline).await.map_err(error)?;
        yolo_capnp_codec::from_value(value).map_err(ProtocolError::Decode)
    }
    pub async fn wait_event(&mut self) -> Result<EventEnvelope, ProtocolError> {
        let events = self
            .events
            .as_mut()
            .ok_or(ProtocolError::ConnectionPoisoned)?;
        let value = events.next_unbounded().await.map_err(error)?;
        yolo_capnp_codec::from_value(value).map_err(ProtocolError::Decode)
    }
    pub fn close(&mut self) {
        self.events = None;
        self.connection.close();
    }
}

fn error(error: yolo_capnp_rpc::Error) -> ProtocolError {
    match error {
        yolo_capnp_rpc::Error::Closed => ProtocolError::EndOfStream,
        yolo_capnp_rpc::Error::OutcomeUnknown => ProtocolError::OutcomeUnknown,
        other => ProtocolError::Io(std::io::Error::other(other.to_string())),
    }
}
