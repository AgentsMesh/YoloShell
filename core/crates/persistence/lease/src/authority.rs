use super::constants::{
    AUTHORIZATION_FAILED_MESSAGE, DEFAULT_TTL_MS, MAX_TTL_MS, MIN_TTL_MS, RENEW_AFTER_DIVISOR, code,
};
use super::errors::lease_error;
use super::{LeaseRequest, LeaseTransition};
use sha2::digest::Output;
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::{Duration, Instant};
use yolo_terminal_model::{WriterCapability, WriterLease, WriterLeaseGrant, now_ms};
use yolo_wire::ErrorBody;

type CapabilityDigest = Output<Sha256>;
const NO_DIGEST_DIFFERENCE: u8 = 0;

#[derive(Clone)]
pub(super) struct ActiveLease {
    pub(super) lease: WriterLease,
    pub(super) deadline: Instant,
    capability_digest: CapabilityDigest,
}

impl fmt::Debug for ActiveLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ActiveLease")
            .field("lease", &self.lease)
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
pub struct LeaseAuthority {
    pub(super) current: Option<ActiveLease>,
    pub(super) next_generation: u64,
}

impl LeaseAuthority {
    pub fn snapshot(&mut self, emit: &mut dyn FnMut(LeaseTransition)) -> Option<WriterLease> {
        self.expire(emit);
        self.current.as_ref().map(|active| active.lease.clone())
    }

    pub fn validate_authorization(
        &mut self,
        generation: u64,
        writer_capability: &WriterCapability,
        emit: &mut dyn FnMut(LeaseTransition),
    ) -> Result<(), ErrorBody> {
        if let Some(expired) = self.expire(emit) {
            return Err(lease_error(
                code::LEASE_EXPIRED,
                "the writer lease has expired",
                Some(&expired),
                None,
            ));
        }
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| lease_error(code::NOT_HOLDER, "writer lease is required", None, None))?;
        if active.lease.generation != generation {
            return Err(lease_error(
                code::STALE_GENERATION,
                "the writer lease generation is stale",
                Some(&active.lease),
                None,
            ));
        }
        if !active.authorizes(writer_capability) {
            return Err(lease_error(
                code::NOT_HOLDER,
                AUTHORIZATION_FAILED_MESSAGE,
                Some(&active.lease),
                None,
            ));
        }
        Ok(())
    }

    pub(super) fn expire(&mut self, emit: &mut dyn FnMut(LeaseTransition)) -> Option<WriterLease> {
        if self
            .current
            .as_ref()
            .is_none_or(|active| Instant::now() < active.deadline)
        {
            return None;
        }
        let expired = self.current.take().expect("lease checked as present").lease;
        emit(LeaseTransition::expired(&expired));
        Some(expired)
    }

    pub(super) fn make_lease(
        &mut self,
        request: &LeaseRequest,
        ttl_ms: u64,
    ) -> Result<ActiveLease, ErrorBody> {
        self.next_generation = self.next_generation.checked_add(1).ok_or_else(|| {
            ErrorBody::new(
                code::GENERATION_OVERFLOW,
                "writer lease generation overflow",
                false,
            )
        })?;
        Ok(ActiveLease {
            lease: WriterLease {
                holder_id: request.holder_id.clone(),
                controller_instance_id: request.controller_instance_id.clone(),
                generation: self.next_generation,
                expires_at_ms: now_ms().saturating_add(ttl_ms as i64),
                ttl_ms,
                renew_after_ms: renew_after_ms(ttl_ms),
            },
            deadline: Instant::now() + Duration::from_millis(ttl_ms),
            capability_digest: capability_digest(&request.writer_capability),
        })
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn expire_for_test(&mut self) {
        if let Some(active) = self.current.as_mut() {
            active.deadline = Instant::now() - super::constants::EXPIRED_DEADLINE_OFFSET;
        }
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn set_wall_expiry_for_test(&mut self, expires_at_ms: i64) {
        if let Some(active) = self.current.as_mut() {
            active.lease.expires_at_ms = expires_at_ms;
        }
    }
}

impl ActiveLease {
    pub(super) fn authorizes(&self, writer_capability: &WriterCapability) -> bool {
        constant_time_eq(
            &self.capability_digest,
            &capability_digest(writer_capability),
        )
    }

    pub(super) fn grant(&self, writer_capability: &WriterCapability) -> WriterLeaseGrant {
        WriterLeaseGrant {
            lease: self.lease.clone(),
            writer_capability: writer_capability.clone(),
        }
    }
}

pub(super) fn requested_ttl(requested: Option<u64>) -> u64 {
    requested
        .unwrap_or(DEFAULT_TTL_MS)
        .clamp(MIN_TTL_MS, MAX_TTL_MS)
}

pub(super) fn renew(active: &mut ActiveLease, ttl_ms: u64) {
    active.lease.ttl_ms = ttl_ms;
    active.lease.renew_after_ms = renew_after_ms(ttl_ms);
    active.lease.expires_at_ms = now_ms().saturating_add(ttl_ms as i64);
    active.deadline = Instant::now() + Duration::from_millis(ttl_ms);
}

fn renew_after_ms(ttl_ms: u64) -> u64 {
    (ttl_ms / RENEW_AFTER_DIVISOR).max(1)
}

fn capability_digest(writer_capability: &WriterCapability) -> CapabilityDigest {
    Sha256::digest(writer_capability.as_bytes())
}

fn constant_time_eq(left: &CapabilityDigest, right: &CapabilityDigest) -> bool {
    left.iter()
        .zip(right)
        .fold(NO_DIGEST_DIFFERENCE, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == NO_DIGEST_DIFFERENCE
}
