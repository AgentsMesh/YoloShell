use crate::lease::constants::{DEFAULT_TTL_MS, MAX_TTL_MS, MIN_TTL_MS, RENEW_AFTER_DIVISOR};
use crate::model::{WriterLease, WriterLeaseGrant, now_ms};
use crate::protocol::{RESPONSE_KIND, RequestEnvelope, ResponseEnvelope, method, to_value};
use crate::worker::LeaseRequest;
use std::time::Duration;
use yolo_capnp_value::Value;

const MALFORMED_EVENT_REQUEST_ID: &str = "malformed-attachment-event";
pub(super) const METHOD_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const METHOD_WAIT_TIMEOUT_DETAIL: &str = "fake Host did not receive the expected RPC";
pub(super) const METHOD_WATCH_CLOSED_DETAIL: &str = "fake Host request watch closed";

pub(super) fn success(request: &RequestEnvelope, payload: Value) -> ResponseEnvelope {
    ResponseEnvelope {
        version: request.version,
        kind: RESPONSE_KIND.into(),
        request_id: request.request_id.clone(),
        method: request.method.clone(),
        ok: true,
        payload,
        error: None,
    }
}

pub(super) fn requested_grant(request: LeaseRequest, generation: u64) -> WriterLeaseGrant {
    requested_grant_with_ttl(request, generation, DEFAULT_TTL_MS)
}

pub(super) fn requested_grant_with_ttl(
    request: LeaseRequest,
    generation: u64,
    ttl_ms: u64,
) -> WriterLeaseGrant {
    let ttl_ms = ttl_ms.clamp(MIN_TTL_MS, MAX_TTL_MS);
    let lease = WriterLease {
        holder_id: request.holder_id,
        controller_instance_id: request.controller_instance_id,
        generation,
        expires_at_ms: now_ms().saturating_add(ttl_ms as i64),
        ttl_ms,
        renew_after_ms: ttl_ms / RENEW_AFTER_DIVISOR,
    };
    WriterLeaseGrant {
        lease,
        writer_capability: request.writer_capability,
    }
}

pub(super) fn malformed_event() -> RequestEnvelope {
    RequestEnvelope::new(
        MALFORMED_EVENT_REQUEST_ID,
        method::HOST_STATUS,
        to_value(&()).unwrap(),
    )
}
