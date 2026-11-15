use super::{FollowUpGate, FragmentedEvent};
use crate::protocol::{
    ErrorBody, EventEnvelope, RequestEnvelope, ResponseEnvelope, delivery, to_value,
};
use crate::worker::{AttachResult, InputRequest, InputResult};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Notify, oneshot};
use yolo_capnp_value::Value;

pub(crate) enum Reply {
    Success(Value),
    Error(ErrorBody),
    Disconnect,
    Pending,
    InputDelivered,
    GatedSuccess(Value, FollowUpGate),
    Lease(u64),
    LeaseWithTtl {
        generation: u64,
        ttl_ms: u64,
    },
    LeaseThenDisconnect(u64),
    LeaseThenPending(u64, Arc<Notify>),
    StoredLease,
    Attachment {
        initial: AttachResult,
        events: Vec<EventEnvelope>,
        gate: Option<FollowUpGate>,
        disconnect: bool,
        malformed_event: bool,
    },
    FragmentedAttachment {
        initial: AttachResult,
        event: FragmentedEvent,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct RenewalReply {
    pub generation: u64,
    pub ttl_ms: u64,
}

impl RenewalReply {
    pub(super) fn for_request(
        self,
        request: &RequestEnvelope,
        lease: Option<&crate::model::WriterLease>,
    ) -> Option<Reply> {
        if request.method != crate::protocol::method::RENEW_WRITER {
            return None;
        }
        let lease = lease?;
        (lease.generation == self.generation && lease.ttl_ms == self.ttl_ms).then_some(
            Reply::LeaseWithTtl {
                generation: self.generation,
                ttl_ms: self.ttl_ms,
            },
        )
    }
}

pub(super) fn input_delivered(request: &RequestEnvelope) -> ResponseEnvelope {
    let input: InputRequest = request.decode_payload().unwrap();
    let result = InputResult {
        input_id: input.input_id,
        duplicate: false,
        delivery: delivery::DELIVERED.into(),
    };
    super::success(request, to_value(&result).unwrap())
}

impl Reply {
    pub(crate) fn value<T: Serialize>(value: &T) -> Self {
        Self::Success(to_value(value).unwrap())
    }

    pub(crate) fn gated_value<T: Serialize>(value: &T, gate: FollowUpGate) -> Self {
        Self::GatedSuccess(to_value(value).unwrap(), gate)
    }

    pub(crate) fn error(code: &str) -> Self {
        Self::Error(ErrorBody::new(code, code, false))
    }

    pub(crate) fn retryable_error(code: &str) -> Self {
        Self::Error(ErrorBody::new(code, code, true))
    }

    pub(crate) fn attachment(initial: AttachResult, events: Vec<EventEnvelope>) -> Self {
        Self::Attachment {
            initial,
            events,
            gate: None,
            disconnect: false,
            malformed_event: false,
        }
    }

    pub(crate) fn closing_attachment(initial: AttachResult) -> Self {
        Self::Attachment {
            initial,
            events: Vec::new(),
            gate: None,
            disconnect: true,
            malformed_event: false,
        }
    }

    pub(crate) fn malformed_attachment(initial: AttachResult) -> Self {
        Self::Attachment {
            initial,
            events: Vec::new(),
            gate: None,
            disconnect: false,
            malformed_event: true,
        }
    }

    pub(crate) fn fragmented_attachment(
        initial: AttachResult,
        event: EventEnvelope,
        ready: oneshot::Receiver<()>,
        fragment_bytes: usize,
        fragment_sent: oneshot::Sender<()>,
        resume: Arc<Notify>,
    ) -> Self {
        Self::FragmentedAttachment {
            initial,
            event: super::FragmentedEvent::new(event, ready, fragment_bytes, fragment_sent, resume),
        }
    }

    pub(crate) fn gated_attachment(
        initial: AttachResult,
        events: Vec<EventEnvelope>,
        gate: super::FollowUpGate,
    ) -> Self {
        Self::Attachment {
            initial,
            events,
            gate: Some(gate),
            disconnect: false,
            malformed_event: false,
        }
    }

    pub(crate) fn gated_closing_attachment(
        initial: AttachResult,
        gate: super::FollowUpGate,
    ) -> Self {
        Self::Attachment {
            initial,
            events: Vec::new(),
            gate: Some(gate),
            disconnect: true,
            malformed_event: false,
        }
    }
}
