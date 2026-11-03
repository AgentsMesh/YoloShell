use crate::{Call, ExportedOrigin, PeerError, Purpose, Signed, constants::*};
use serde::{Deserialize, Serialize};
use yolo_hub_rpc_protocol::{Command, Response};
use yolo_hub_terminal_protocol::{PublicError, TerminalEvent};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
pub enum PeerRequest {
    Discover {
        #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
        request_id: uuid::Uuid,
        version: u16,
        allowed_origins: Vec<yolo_hub_identity::HubId>,
    },
    Execute {
        call: Signed<Call>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum PeerReply {
    Directory {
        #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
        request_id: uuid::Uuid,
        revision: u64,
        origins: Vec<ExportedOrigin>,
    },
    Executed {
        response: Signed<ExecutionReply>,
    },
    Rejected {
        call_fingerprint: [u8; ring::digest::SHA256_OUTPUT_LEN],
        code: PublicError,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReply {
    pub call_fingerprint: [u8; ring::digest::SHA256_OUTPUT_LEN],
    pub response: Response,
}
impl Signed<ExecutionReply> {
    pub fn validate_for(&self, call: &Signed<Call>) -> Result<(), PeerError> {
        self.verify(Purpose::Response, call.payload.authority.origin())?;
        if self.payload.call_fingerprint != call.fingerprint()? {
            return Err(PeerError::Correlation);
        }
        self.payload.response.validate_for(&call.payload.request)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalUpdate {
    pub call_fingerprint: [u8; ring::digest::SHA256_OUTPUT_LEN],
    pub sequence: u64,
    pub issued_at_ms: u64,
    pub event: TerminalEvent,
}
impl Signed<TerminalUpdate> {
    pub fn validate_for(
        &self,
        call: &Signed<Call>,
        expected_sequence: u64,
        now: u64,
    ) -> Result<(), PeerError> {
        self.verify(Purpose::Terminal, call.payload.authority.origin())?;
        let event = &self.payload;
        if event.call_fingerprint != call.fingerprint()?
            || event.sequence != expected_sequence
            || event.sequence < FIRST_STREAM_SEQUENCE
        {
            return Err(PeerError::Correlation);
        }
        if now.saturating_add(MAX_CLOCK_SKEW_MS) < event.issued_at_ms || now.saturating_sub(event.issued_at_ms) > MAX_TERMINAL_LIFETIME_MS {
            return Err(PeerError::Expired);
        }
        let Command::Attach { request } = &call.payload.request.fields().command else {
            return Err(PeerError::Correlation);
        };
        if event.event.target() != request.target() {
            return Err(PeerError::Correlation);
        }
        event.event.validate()?;
        Ok(())
    }
}
