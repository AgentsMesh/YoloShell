use crate::model::{JoinOutcome, JoinRequest, validate_frame_size};
use crate::{MAX_JOIN_FRAME_BYTES, WireError};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn read_request(
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<JoinRequest, WireError> {
    let request: JoinRequest = read_frame(stream).await?;
    request.validate().map_err(WireError::Rejected)?;
    Ok(request)
}

pub async fn read_outcome(
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<JoinOutcome, WireError> {
    let outcome: JoinOutcome = read_frame(stream).await?;
    outcome.validate().map_err(WireError::Rejected)?;
    Ok(outcome)
}

pub async fn write_request(
    stream: &mut (impl AsyncWrite + Unpin),
    request: &JoinRequest,
) -> Result<(), WireError> {
    request.validate().map_err(WireError::Rejected)?;
    write_frame(stream, request).await
}

pub async fn write_outcome(
    stream: &mut (impl AsyncWrite + Unpin),
    outcome: &JoinOutcome,
) -> Result<(), WireError> {
    outcome.validate().map_err(WireError::Rejected)?;
    write_frame(stream, outcome).await
}

async fn read_frame<T: serde::de::DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<T, WireError> {
    let length = stream.read_u32().await.map_err(WireError::Io)?;
    let length = usize::try_from(length).map_err(|_| WireError::Size)?;
    validate_frame_size(length)?;
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await.map_err(WireError::Io)?;
    serde_json::from_slice(&bytes).map_err(|_| WireError::Codec)
}

async fn write_frame<T: serde::Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
) -> Result<(), WireError> {
    let bytes = serde_json::to_vec(value).map_err(|_| WireError::Codec)?;
    if bytes.is_empty() || bytes.len() > MAX_JOIN_FRAME_BYTES {
        return Err(WireError::Size);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| WireError::Size)?;
    stream.write_u32(length).await.map_err(WireError::Io)?;
    stream.write_all(&bytes).await.map_err(WireError::Io)?;
    stream.flush().await.map_err(WireError::Io)
}
