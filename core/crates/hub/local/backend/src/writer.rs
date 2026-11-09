use crate::{
    LocalBackend,
    errors::map_error,
    writer_conversion::{internal_grant, public_grant, runtime},
};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{
    InputRequest, LeaseGrant, LeaseMode, LeaseRequest, PublicError, ResizeRequest,
};
use yolo_terminal_model::WriterCapability;

impl LocalBackend {
    pub(crate) async fn acquire(&self, request: LeaseRequest) -> Result<Reply, PublicError> {
        request.validate()?;
        let capability = WriterCapability::generate();
        let target = runtime(request.target);
        let principal = request.principal;
        let grant = match request.mode {
            LeaseMode::Acquire {} => {
                self.client
                    .acquire_writer_for_instance(
                        target,
                        principal.client_id.to_string(),
                        Some(principal.controller_instance_id.to_string()),
                        capability,
                        None,
                    )
                    .await
            }
            LeaseMode::TakeControl {
                observed_generation,
            } => {
                self.client
                    .take_over_writer_for_instance(
                        target,
                        principal.client_id.to_string(),
                        Some(principal.controller_instance_id.to_string()),
                        Some(observed_generation),
                        capability,
                        None,
                    )
                    .await
            }
        }
        .map_err(map_error)?;
        Ok(Reply::Lease {
            grant: public_grant(request.target, principal, grant)?,
        })
    }

    pub(crate) async fn renew(&self, grant: LeaseGrant) -> Result<Reply, PublicError> {
        let current = self
            .client
            .renew_writer(
                &internal_grant(grant)?,
                runtime(grant.fields().target),
                None,
            )
            .await
            .map_err(map_error)?;
        Ok(Reply::Lease {
            grant: public_grant(grant.fields().target, grant.fields().principal, current)?,
        })
    }

    pub(crate) async fn release(&self, grant: LeaseGrant) -> Result<Reply, PublicError> {
        self.client
            .release_writer(&internal_grant(grant)?, runtime(grant.fields().target))
            .await
            .map_err(map_error)?;
        Ok(Reply::Completed {})
    }

    pub(crate) async fn input(&self, request: &InputRequest) -> Result<Reply, PublicError> {
        let fields = request.fields();
        let grant = internal_grant(fields.grant)?;
        let target = runtime(fields.grant.fields().target);
        let worker_request = yolo_runtime_protocol::InputRequest {
            terminal_id: target.terminal_id,
            runtime_epoch: target.runtime_epoch,
            generation: grant.generation,
            writer_capability: grant.writer_capability,
            input_id: fields.request_id,
            data: fields.data.clone(),
        };
        let connection_slot = self.host_connections.slot(target.terminal_id).await;
        let mut cached = connection_slot.lock().await;
        // A cancelled operation drops its connection instead of retaining a
        // poisoned stream. Only a validated, confirmed success returns it.
        let mut connection = match (!cached.idle(std::time::Instant::now()))
            .then(|| cached.connection.take())
            .flatten()
        {
            Some(connection) => connection,
            None => self.client.connect().await.map_err(map_error)?,
        };
        let result = connection
            .input_with_id(&worker_request)
            .await
            .map_err(map_error)?;
        if result.input_id != fields.request_id
            || result.delivery != yolo_wire::delivery::DELIVERED
        {
            return Err(PublicError::OutcomeUnknown);
        }
        cached.connection = Some(connection);
        cached.last_used = std::time::Instant::now();
        Ok(Reply::Input {
            duplicate: result.duplicate,
        })
    }

    pub(crate) async fn resize(&self, request: ResizeRequest) -> Result<Reply, PublicError> {
        request.validate()?;
        let grant = internal_grant(request.grant)?;
        self.client
            .resize(
                runtime(request.grant.fields().target),
                grant.generation,
                &grant.writer_capability,
                request.size.cols(),
                request.size.rows(),
            )
            .await
            .map_err(map_error)?;
        Ok(Reply::Completed {})
    }
}
