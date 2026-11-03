use super::authority::{LeaseAuthority, renew, requested_ttl};
use super::constants::code;
use super::errors::{lease_error, validate_request};
use super::{LeaseReason, LeaseRequest, LeaseTransition};
use yolo_terminal_model::{WriterLease, WriterLeaseGrant};
use yolo_wire::ErrorBody;

impl LeaseAuthority {
    pub fn renew(
        &mut self,
        request: &LeaseRequest,
        emit: &mut dyn FnMut(LeaseTransition),
    ) -> Result<WriterLeaseGrant, ErrorBody> {
        if let Some(expired) = self.expire(emit) {
            return Err(lease_error(
                code::LEASE_EXPIRED,
                "the writer lease has expired",
                Some(&expired),
                Some(request),
            ));
        }
        let active = self.current.as_mut().ok_or_else(|| {
            lease_error(
                code::NOT_HOLDER,
                "writer lease does not exist",
                None,
                Some(request),
            )
        })?;
        validate_request(&active.lease, request)?;
        self.validate_authorization(
            request
                .generation
                .expect("lease request generation validated"),
            &request.writer_capability,
            emit,
        )?;
        let active = self
            .current
            .as_mut()
            .expect("lease authorization validated");
        renew(active, requested_ttl(request.ttl_ms));
        Ok(active.grant(&request.writer_capability))
    }

    pub fn release(
        &mut self,
        request: &LeaseRequest,
        emit: &mut dyn FnMut(LeaseTransition),
    ) -> Result<(), ErrorBody> {
        if let Some(expired) = self.expire(emit) {
            return Err(lease_error(
                code::LEASE_EXPIRED,
                "the writer lease has expired",
                Some(&expired),
                Some(request),
            ));
        }
        let active = self.current.as_ref().ok_or_else(|| {
            lease_error(
                code::NOT_HOLDER,
                "writer lease does not exist",
                None,
                Some(request),
            )
        })?;
        validate_request(&active.lease, request)?;
        self.validate_authorization(
            request
                .generation
                .expect("lease request generation validated"),
            &request.writer_capability,
            emit,
        )?;
        let active = self
            .current
            .as_ref()
            .expect("lease authorization validated");
        emit(LeaseTransition::revoked(
            &active.lease,
            LeaseReason::Released,
        ));
        self.current = None;
        Ok(())
    }

    pub fn revoke(
        &mut self,
        reason: LeaseReason,
        emit: &mut dyn FnMut(LeaseTransition),
    ) -> Option<WriterLease> {
        let lease = self.current.take()?.lease;
        emit(LeaseTransition::revoked(&lease, reason));
        Some(lease)
    }
}
