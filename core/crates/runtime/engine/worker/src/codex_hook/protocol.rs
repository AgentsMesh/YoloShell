use super::constants::{
    CODEX_ACK_PROTOCOL, CODEX_FRAME_HEADER_BYTES, CODEX_MAX_FRAME_BYTES, CODEX_MIN_FRAME_BYTES,
    CODEX_REQUEST_READ_ERROR,
};
use crate::agent::hooks::HookEnvelope;
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CodexIngressRequest {
    Event(HookEnvelope),
    Native(crate::codex_hooks::CodexNativeRequest),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CodexHookAck {
    pub schema_version: u16,
    pub accepted: bool,
    #[serde(default)]
    pub event_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub sequence: Option<u64>,
    pub code: String,
}

impl CodexHookAck {
    pub(super) fn accepted(event: &HookEnvelope, replay: bool) -> Self {
        Self {
            schema_version: crate::codex_hooks::ACK_SCHEMA_VERSION,
            accepted: true,
            event_id: Some(event.event_id),
            sequence: Some(event.sequence.0),
            code: (if replay {
                crate::codex_hooks::ACK_CODE_REPLAY
            } else {
                crate::codex_hooks::ACK_CODE_ACCEPTED
            })
            .to_owned(),
        }
    }

    pub(super) fn rejected(event: Option<&HookEnvelope>, code: &str) -> Self {
        Self {
            schema_version: crate::codex_hooks::ACK_SCHEMA_VERSION,
            accepted: false,
            event_id: event.map(|value| value.event_id),
            sequence: event.map(|value| value.sequence.0),
            code: code.to_owned(),
        }
    }

    pub(super) fn protocol_rejection() -> Self {
        Self::rejected(None, CODEX_ACK_PROTOCOL)
    }
}

pub(super) async fn read_envelope(stream: &mut LocalUnixStream) -> Result<HookEnvelope> {
    read_frame(stream).await.context(CODEX_REQUEST_READ_ERROR)
}

pub(super) async fn read_request(stream: &mut LocalUnixStream) -> Result<CodexIngressRequest> {
    let payload = read_frame_payload(stream).await?;
    decode_request(&payload).context(CODEX_REQUEST_READ_ERROR)
}

pub(super) async fn write_ack(stream: &mut LocalUnixStream, ack: &CodexHookAck) -> Result<()> {
    write_frame(stream, ack)
        .await
        .context(super::constants::CODEX_ACK_WRITE_ERROR)
}

async fn read_frame<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: serde::de::DeserializeOwned,
{
    let payload = read_frame_payload(reader).await?;
    yolo_capnp_codec::decode(&payload).context(CODEX_REQUEST_READ_ERROR)
}

async fn read_frame_payload<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; CODEX_FRAME_HEADER_BYTES];
    reader.read_exact(&mut header).await.map_err(frame_error)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(
        (CODEX_MIN_FRAME_BYTES..=CODEX_MAX_FRAME_BYTES).contains(&length),
        CODEX_REQUEST_READ_ERROR
    );
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).await.map_err(frame_error)?;
    Ok(payload)
}

fn decode_request(payload: &[u8]) -> Result<CodexIngressRequest> {
    if let Ok(request) = yolo_capnp_codec::decode::<crate::codex_hooks::CodexNativeRequest>(payload) {
        request.validate()?;
        return Ok(CodexIngressRequest::Native(request));
    }
    yolo_capnp_codec::decode::<HookEnvelope>(payload)
        .map(CodexIngressRequest::Event)
        .context(CODEX_REQUEST_READ_ERROR)
}

async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = yolo_capnp_codec::encode(value).context(super::constants::CODEX_ACK_WRITE_ERROR)?;
    ensure!(
        payload.len() <= CODEX_MAX_FRAME_BYTES,
        super::constants::CODEX_ACK_WRITE_ERROR
    );
    let length = u32::try_from(payload.len()).context(super::constants::CODEX_ACK_WRITE_ERROR)?;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .context(super::constants::CODEX_ACK_WRITE_ERROR)?;
    writer
        .write_all(&payload)
        .await
        .context(super::constants::CODEX_ACK_WRITE_ERROR)
}

fn frame_error(error: std::io::Error) -> anyhow::Error {
    anyhow::Error::new(error).context(CODEX_REQUEST_READ_ERROR)
}
