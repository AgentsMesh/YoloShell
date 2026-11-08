use super::constants::REMOTE_REQUEST_FAILED_MESSAGE;
use super::constants::{REQUIRED_RUNTIME_CAPABILITIES, RUNTIME_CLIENT_NAME};
use super::protocol::validate_runtime;
use super::{BoxedRemoteChannel, RemoteDeadline, RemoteError, RemoteRuntimeSpec};
use crate::host::{HelloRequest, HelloResult};
use crate::model::HostStatus;
use crate::protocol::{
    EventEnvelope, ProtocolError, RequestEnvelope, ResponseEnvelope, to_value,
    validate_attachment_event, validate_response,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

pub async fn probe_runtime_channel(
    stream: BoxedRemoteChannel,
    runtime: &RemoteRuntimeSpec,
    deadline: RemoteDeadline,
) -> Result<yolo_runtime_rpc::RuntimeConnection, RemoteError> {
    let mut rpc = RemoteRpc::new(stream);
    let hello: HelloResult = rpc
        .request_until(
            crate::protocol::method::HELLO,
            &HelloRequest {
                client_name: Some(RUNTIME_CLIENT_NAME.into()),
                protocol_version: crate::model::PROTOCOL_VERSION,
                capabilities: REQUIRED_RUNTIME_CAPABILITIES
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
            deadline,
        )
        .await?;
    let status: HostStatus = rpc
        .request_until(crate::protocol::method::HOST_STATUS, &(), deadline)
        .await?;
    validate_runtime(&hello.server_name, hello.protocol_version, &status, runtime)?;
    rpc.into_connection()
}

pub(crate) struct RemoteRpc {
    stream: Option<BoxedRemoteChannel>,
    connection: Option<yolo_runtime_rpc::RuntimeConnection>,
}

impl RemoteRpc {
    pub fn new(stream: BoxedRemoteChannel) -> Self {
        Self {
            stream: Some(stream),
            connection: None,
        }
    }

    pub async fn raw_until<T: Serialize>(
        &mut self,
        method: &str,
        payload: &T,
        deadline: RemoteDeadline,
    ) -> Result<ResponseEnvelope, RemoteError> {
        let request = RequestEnvelope::new(Uuid::new_v4().to_string(), method, to_value(payload)?);
        let exchange = async {
            self.connect(deadline).await?;
            self.connection
                .as_mut()
                .ok_or(ProtocolError::ConnectionPoisoned)?
                .exchange(&request, deadline.instant())
                .await
                .map_err(Into::into)
        };
        let response = match exchange.await {
            Ok(response) => response,
            Err(error) => return Err(self.poison(error)),
        };
        if let Err(error) = validate_response(&request, &response) {
            return Err(self.poison(RemoteError::Protocol(error)));
        }
        Ok(response)
    }

    pub async fn request_until<T: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        payload: &T,
        deadline: RemoteDeadline,
    ) -> Result<R, RemoteError> {
        let response = self.raw_until(method, payload, deadline).await?;
        match decode_response(response) {
            Err(error @ RemoteError::Protocol(_)) => Err(self.poison(error)),
            result => result,
        }
    }

    pub async fn next_event(
        &mut self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<EventEnvelope, RemoteError> {
        let Some(connection) = self.connection.as_mut() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let event = match connection.wait_event().await {
            Ok(event) => event,
            Err(error) => return Err(self.poison(error.into())),
        };
        if let Err(error) = validate_attachment_event(&event, terminal_id, runtime_epoch) {
            return Err(self.poison(RemoteError::Protocol(error)));
        }
        Ok(event)
    }

    pub async fn shutdown_until(&mut self, deadline: RemoteDeadline) -> Result<(), RemoteError> {
        if let Some(mut connection) = self.connection.take() {
            connection.close();
            return Ok(());
        }
        use tokio::io::AsyncWriteExt;
        let Some(stream) = self.stream.as_mut() else {
            return Err(ProtocolError::ConnectionPoisoned.into());
        };
        let result = deadline
            .run(async { stream.shutdown().await.map_err(Into::into) })
            .await;
        self.stream = None;
        result
    }

    async fn connect(&mut self, deadline: RemoteDeadline) -> Result<(), RemoteError> {
        if self.connection.is_none() {
            let stream = self
                .stream
                .take()
                .ok_or(ProtocolError::ConnectionPoisoned)?;
            self.connection =
                Some(yolo_runtime_rpc::RuntimeConnection::open(stream, deadline.instant()).await?);
        }
        Ok(())
    }
    fn into_connection(mut self) -> Result<yolo_runtime_rpc::RuntimeConnection, RemoteError> {
        self.connection
            .take()
            .ok_or_else(|| ProtocolError::ConnectionPoisoned.into())
    }

    fn poison(&mut self, error: RemoteError) -> RemoteError {
        self.stream = None;
        if let Some(mut connection) = self.connection.take() {
            connection.close();
        }
        error
    }
}

pub(crate) fn decode_response<T: DeserializeOwned>(
    response: ResponseEnvelope,
) -> Result<T, RemoteError> {
    successful(response)?.decode_payload().map_err(Into::into)
}

fn successful(response: ResponseEnvelope) -> Result<ResponseEnvelope, RemoteError> {
    if !response.ok {
        return Err(RemoteError::Host(response.error.unwrap_or_else(|| {
            crate::protocol::ErrorBody::internal(REMOTE_REQUEST_FAILED_MESSAGE)
        })));
    }
    Ok(response)
}
