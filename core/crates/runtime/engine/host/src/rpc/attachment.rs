use super::super::*;
use yolo_capnp_rpc::{EventSource, Future, Value};

impl Host {
    pub(in crate::host) async fn rpc_attachment(
        &self,
        request: &RequestEnvelope,
    ) -> std::result::Result<(ResponseEnvelope, Box<dyn EventSource>, OwnedMutexGuard<()>), ErrorBody>
    {
        self.request_drain.ensure_attachment_open()?;
        let capacity = crate::ingress::try_admit(&self.attachment_clients)
            .ok_or_else(|| ErrorBody::conflict(HOST_ATTACHMENT_LIMIT_MESSAGE))?;
        self.request_drain.ensure_attachment_open()?;
        let payload: AttachRequest = request
            .decode_payload()
            .map_err(|error| ErrorBody::invalid(error.to_string()))?;
        let publication = self.session_locks.lock(payload.terminal_id).await;
        let record = self.live_runtime_record(payload.terminal_id).await?;
        let (mut worker, authority) = self.attachment_worker_stream(&record).await?;
        let response = exchange_worker_request(&mut worker, request)
            .await
            .map_err(WorkerRpcError::into_error_body)?;
        if !response.ok {
            return Err(response
                .error
                .unwrap_or_else(|| ErrorBody::internal(WORKER_ATTACHMENT_PROTOCOL_INVALID)));
        }
        validate_worker_attachment(request, &response, &record)?;
        self.validate_attachment_authority(&record, authority)
            .await?;
        Ok((
            response,
            Box::new(Events {
                worker,
                record,
                _capacity: capacity,
            }),
            publication,
        ))
    }
}

struct Events {
    worker: crate::worker::WorkerConnection,
    record: RuntimeRecord,
    _capacity: tokio::sync::OwnedSemaphorePermit,
}
impl EventSource for Events {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async move {
            let event = self
                .worker
                .next_event()
                .await
                .map_err(|error| match error {
                    WorkerRpcError::Closed => yolo_capnp_rpc::Error::Closed,
                    other => yolo_capnp_rpc::Error::Remote(other.to_string()),
                })?;
            crate::protocol::validate_attachment_event(
                &event,
                self.record.terminal_id,
                self.record.runtime_epoch,
            )
            .map_err(|error| yolo_capnp_rpc::Error::Remote(error.to_string()))?;
            yolo_capnp_codec::to_value(&event).map_err(Into::into)
        })
    }
}
