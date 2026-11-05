use crate::MAX_JOIN_FRAME_BYTES;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;

pub const JOIN_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinRequest {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub invite_id: Uuid,
    pub peer: PeerIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinReply {
    pub schema_version: u16,
    pub hub: PeerIdentity,
    pub revision: u64,
    pub applied: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum JoinOutcome {
    Accepted { reply: JoinReply },
    Rejected { code: JoinErrorCode },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum JoinErrorCode {
    #[error("the enrollment request is invalid")]
    InvalidRequest,
    #[error("the pairing code is unknown, expired or already used")]
    Unauthorized,
    #[error("the Hub configuration conflicts with this enrollment")]
    Conflict,
    #[error("the Hub cannot admit this enrollment right now")]
    Unavailable,
    #[error("this Hub does not support enrollment")]
    Unsupported,
}

impl JoinRequest {
    pub fn validate(&self) -> Result<(), JoinErrorCode> {
        if self.schema_version != JOIN_SCHEMA_VERSION
            || self.request_id.is_nil()
            || self.invite_id.is_nil()
        {
            return Err(JoinErrorCode::InvalidRequest);
        }
        Ok(())
    }
}

impl JoinOutcome {
    pub fn validate(&self) -> Result<(), JoinErrorCode> {
        if let Self::Accepted { reply } = self {
            if reply.schema_version != JOIN_SCHEMA_VERSION || reply.revision == 0 {
                return Err(JoinErrorCode::InvalidRequest);
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_frame_size(length: usize) -> Result<(), crate::WireError> {
    if length == 0 || length > MAX_JOIN_FRAME_BYTES {
        return Err(crate::WireError::Size);
    }
    Ok(())
}
