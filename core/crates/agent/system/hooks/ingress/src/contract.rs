use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

pub const SCHEMA_VERSION: u16 = 1;
pub const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
pub use super::constants::{ACCEPTED, NOT_READY, REJECTED};
const INVALID_SCOPE: &str = "hook scope identity is invalid";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

impl HookProcessIdentity {
    pub fn is_valid(self) -> bool {
        self.pid != u32::default() && self.start_time != u64::default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookScope {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub agent: HookProcessIdentity,
}

impl HookScope {
    pub fn is_valid(self) -> bool {
        !self.terminal_id.is_nil() && !self.runtime_epoch.is_nil() && self.agent.is_valid()
    }
}

/// One provider hook invocation. `provider` names the codec; `payload` is the
/// provider's own stdin JSON, kept opaque here.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HookRequest {
    pub schema_version: u16,
    pub provider: String,
    pub scope: HookScope,
    pub event_sequence: u64,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub event_id: Uuid,
    pub payload: serde_json::Value,
}

impl HookRequest {
    pub fn is_valid(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && self.provider.len() <= super::constants::MAX_PROVIDER_BYTES
            && !self.provider.is_empty()
            && self.scope.is_valid()
            && self.event_sequence >= super::constants::FIRST_SEQUENCE
            && !self.event_id.is_nil()
            && serde_json::to_vec(&self.payload)
                .map(|bytes| bytes.len() <= MAX_PAYLOAD_BYTES)
                .unwrap_or(false)
    }

    pub fn ensure_scope(&self) -> Result<HookScope> {
        anyhow::ensure!(self.scope.is_valid(), INVALID_SCOPE);
        Ok(self.scope)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HookResponse {
    pub schema_version: u16,
    pub accepted: bool,
    pub code: String,
    pub scope: HookScope,
    pub event_sequence: u64,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub event_id: Uuid,
    pub invocation_id: Option<Uuid>,
    pub revision: Option<u64>,
}

impl HookResponse {
    pub fn rejection(request: &HookRequest) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            accepted: false,
            code: REJECTED.to_owned(),
            scope: request.scope,
            event_sequence: request.event_sequence,
            event_id: request.event_id,
            invocation_id: None,
            revision: None,
        }
    }

    pub fn acknowledges(&self, request: &HookRequest) -> bool {
        self.accepted
            && self.code == ACCEPTED
            && self.scope == request.scope
            && self.event_sequence == request.event_sequence
            && self.event_id == request.event_id
    }
}

pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(value)?;
    anyhow::ensure!(payload.len() <= MAX_FRAME_BYTES, "hook frame is too large");
    let length = u32::try_from(payload.len()).map_err(|_| anyhow::anyhow!("hook frame is too large"))?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    Ok(())
}

pub async fn read_frame<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: for<'de> Deserialize<'de>,
{
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    anyhow::ensure!(length <= MAX_FRAME_BYTES, "hook frame is too large");
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).await?;
    Ok(serde_json::from_slice(&payload)?)
}
