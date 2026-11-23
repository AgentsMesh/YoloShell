use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_terminal_model::{WriterCapability, WriterLease};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseRequest {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    pub holder_id: String,
    pub ttl_ms: Option<u64>,
    #[serde(default)]
    pub generation: Option<u64>,
    #[serde(default)]
    pub controller_instance_id: Option<String>,
    pub writer_capability: WriterCapability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseTransitionKind {
    Acquired,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseReason {
    ControllerReplaced,
    TakenOver,
    Released,
    Expired,
    SessionEnded,
}

#[derive(Debug, Clone)]
pub struct LeaseTransition {
    pub kind: LeaseTransitionKind,
    pub lease: WriterLease,
    pub reason: Option<LeaseReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseEventPayload {
    pub holder_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_instance_id: Option<String>,
    pub generation: u64,
    pub expires_at_ms: i64,
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<LeaseReason>,
}

impl LeaseEventPayload {
    pub fn new(lease: WriterLease, reason: Option<LeaseReason>) -> Self {
        Self {
            holder_id: lease.holder_id,
            controller_instance_id: lease.controller_instance_id,
            generation: lease.generation,
            expires_at_ms: lease.expires_at_ms,
            ttl_ms: lease.ttl_ms,
            renew_after_ms: lease.renew_after_ms,
            reason,
        }
    }
}

impl From<LeaseTransition> for LeaseEventPayload {
    fn from(transition: LeaseTransition) -> Self {
        Self::new(transition.lease, transition.reason)
    }
}

impl LeaseTransition {
    pub(super) fn acquired(lease: &WriterLease) -> Self {
        Self {
            kind: LeaseTransitionKind::Acquired,
            lease: lease.clone(),
            reason: None,
        }
    }

    pub(super) fn revoked(lease: &WriterLease, reason: LeaseReason) -> Self {
        Self {
            kind: LeaseTransitionKind::Revoked,
            lease: lease.clone(),
            reason: Some(reason),
        }
    }

    pub(super) fn expired(lease: &WriterLease) -> Self {
        Self {
            kind: LeaseTransitionKind::Expired,
            lease: lease.clone(),
            reason: Some(LeaseReason::Expired),
        }
    }
}
