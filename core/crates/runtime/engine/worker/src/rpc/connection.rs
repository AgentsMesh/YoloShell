use super::super::{RequestEnvelope, ResponseEnvelope, WorkerRpcError};
use tokio::time::Instant;
use yolo_capnp_rpc::{Connection, Method, Protocol, RemoteEvents};

pub struct WorkerConnection {
    connection: Connection,
    events: Option<RemoteEvents>,
}

impl std::fmt::Debug for WorkerConnection {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output
            .debug_struct("WorkerConnection")
            .field("attached", &self.events.is_some())
            .finish_non_exhaustive()
    }
}

impl WorkerConnection {
    pub(in crate::worker) async fn open(
        stream: super::super::UnixStream,
    ) -> Result<Self, WorkerRpcError> {
        let connection = Connection::open(
            stream,
            Protocol::Endpoint,
            Instant::now() + super::super::WORKER_AUTH_TIMEOUT,
        )
        .await
        .map_err(WorkerRpcError::from_rpc)?;
        Ok(Self {
            connection,
            events: None,
        })
    }

    pub async fn exchange(
        &mut self,
        request: &RequestEnvelope,
    ) -> Result<ResponseEnvelope, WorkerRpcError> {
        let deadline = Instant::now() + super::super::rpc_timeout(&request.method);
        let reply = self
            .connection
            .request_deferred(
                Method::Call,
                yolo_capnp_codec::to_value(request).map_err(WorkerRpcError::protocol)?,
                deadline,
            )
            .await
            .map_err(WorkerRpcError::from_rpc)?;
        let response =
            yolo_capnp_codec::from_value(reply.value).map_err(WorkerRpcError::protocol)?;
        super::super::validate_response(request, &response)
            .map_err(WorkerRpcError::from_protocol)?;
        if let Some(receipt) = reply.receipt {
            if request.method == crate::protocol::method::TERMINAL_INPUT {
                tokio::spawn(async move {
                    let _ = receipt.confirm(deadline).await;
                });
            } else {
                receipt
                    .confirm(deadline)
                    .await
                    .map_err(WorkerRpcError::from_rpc)?;
            }
        }
        self.events = reply.events;
        Ok(response)
    }

    pub async fn next_event(&mut self) -> Result<crate::protocol::EventEnvelope, WorkerRpcError> {
        let events = self
            .events
            .as_mut()
            .ok_or_else(|| WorkerRpcError::protocol(super::CLOSED_RPC))?;
        let value = events
            .next_unbounded()
            .await
            .map_err(WorkerRpcError::from_rpc)?;
        yolo_capnp_codec::from_value(value).map_err(WorkerRpcError::protocol)
    }
}
