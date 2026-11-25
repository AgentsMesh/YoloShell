use crate::{LocalBackend, errors::map_error, writer_conversion::runtime};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{LeaseObservation, LeaseObservationFields, PublicError};
use yolo_terminal_model::WriterLease;

impl LocalBackend {
    pub async fn observe_writer(
        &self,
        target: RuntimeTarget,
    ) -> Result<Option<LeaseObservation>, PublicError> {
        if target.address.hub_id != self.hub {
            return Err(PublicError::OriginMismatch);
        }
        self.client
            .get_writer_lease(runtime(target))
            .await
            .map_err(map_error)?
            .map(|lease| observation(target, lease))
            .transpose()
    }
}

pub(crate) fn observation(
    target: RuntimeTarget,
    lease: WriterLease,
) -> Result<LeaseObservation, PublicError> {
    LeaseObservation::new(LeaseObservationFields {
        target,
        generation: lease.generation,
        holder_id: lease.holder_id,
        controller_instance_id: lease.controller_instance_id,
        expires_at_ms: lease.expires_at_ms,
    })
}
