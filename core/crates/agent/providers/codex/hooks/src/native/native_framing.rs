use super::{CodexNativeAck, CodexNativeRequest};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};

const REQUEST_FRAME: &str = "Codex native callback frame is invalid";
const ACK_FRAME: &str = "Codex native callback acknowledgement frame is invalid";
pub const CODEX_NATIVE_MAX_REQUEST_BYTES: usize = super::constants::MAX_DELIVERY_FRAME_BYTES;

pub async fn read_native_request<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<CodexNativeRequest> {
    let request: CodexNativeRequest =
        super::wire_framing::read_frame(reader, CODEX_NATIVE_MAX_REQUEST_BYTES, REQUEST_FRAME)
            .await?;
    request.validate()?;
    Ok(request)
}

pub async fn write_native_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    request: &CodexNativeRequest,
) -> Result<()> {
    request.validate()?;
    super::wire_framing::write_frame(
        writer,
        request,
        CODEX_NATIVE_MAX_REQUEST_BYTES,
        REQUEST_FRAME,
    )
    .await
}

pub async fn read_native_ack<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CodexNativeAck> {
    super::wire_framing::read_frame(reader, super::constants::MAX_ACK_BYTES, ACK_FRAME).await
}

pub async fn write_native_ack<W: AsyncWrite + Unpin>(
    writer: &mut W,
    ack: &CodexNativeAck,
) -> Result<()> {
    super::wire_framing::write_frame(writer, ack, super::constants::MAX_ACK_BYTES, ACK_FRAME).await
}
