use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_crypto::Sha256Digest;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeControlOutcome {
    /// The provider's lifecycle reported that it accepted the input: a turn
    /// started after the transport write.
    Accepted,
    /// The provider reported the turn ended after an interrupt.
    Confirmed,
    /// The transport write succeeded, but no provider effect was observed.
    Delivered,
    NotDelivered,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeControlResult {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::sha256_digest::compact")]
    pub command_digest: Sha256Digest,
    pub outcome: NativeControlOutcome,
}

impl NativeControlResult {
    pub fn matches(&self, session: Uuid, epoch: Uuid, request: Uuid, digest: Sha256Digest) -> bool {
        self.terminal_id == session
            && self.runtime_epoch == epoch
            && self.request_id == request
            && self.command_digest == digest
    }
}
