use super::authority::{LeaseAuthority, renew, requested_ttl};
use super::constants::{AUTHORIZATION_FAILED_MESSAGE, code};
use super::errors::{instance_changed, lease_error};
use super::{LeaseReason, LeaseRequest, LeaseTransition};
use yolo_terminal_model::WriterLeaseGrant;
use yolo_wire::ErrorBody;

impl LeaseAuthority {
    pub fn acquire(
        &mut self,
        request: &LeaseRequest,
        takeover: bool,
        emit: &mut dyn FnMut(LeaseTransition),
    ) -> Result<WriterLeaseGrant, ErrorBody> {
        let ttl_ms = requested_ttl(request.ttl_ms);
        self.expire(emit);

        if let Some(active) = self.current.as_mut() {
            let same_holder = active.lease.holder_id == request.holder_id;
            let changed = instance_changed(
                &active.lease.controller_instance_id,
                &request.controller_instance_id,
            );
            if same_holder && !changed {
                if active.authorizes(&request.writer_capability) {
                    renew(active, ttl_ms);
                    return Ok(active.grant(&request.writer_capability));
                }
                if !takeover {
                    return Err(lease_error(
                        code::NOT_HOLDER,
                        AUTHORIZATION_FAILED_MESSAGE,
                        Some(&active.lease),
                        Some(request),
                    ));
                }
            }
            if !takeover {
                return Err(lease_error(
                    code::HELD_BY_OTHER,
                    "writer lease is held by another controller",
                    Some(&active.lease),
                    Some(request),
                ));
            }
            if request.generation != Some(active.lease.generation) {
                return Err(lease_error(
                    code::STALE_GENERATION,
                    "the observed writer lease generation is stale",
                    Some(&active.lease),
                    Some(request),
                ));
            }
            let reason = if same_holder {
                LeaseReason::ControllerReplaced
            } else {
                LeaseReason::TakenOver
            };
            emit(LeaseTransition::revoked(&active.lease, reason));
        }

        let acquired = self.make_lease(request, ttl_ms)?;
        emit(LeaseTransition::acquired(&acquired.lease));
        let grant = acquired.grant(&request.writer_capability);
        self.current = Some(acquired);
        Ok(grant)
    }
}
