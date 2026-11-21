use crate::LinkError;
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};
use yolo_capnp_codec::{decode, encode};

const MAX_HANDSHAKE_BYTES: usize = 4 * 1024;

pub(crate) async fn read<T: DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
    deadline: Instant,
) -> Result<T, LinkError> {
    let length = timeout_at(deadline, stream.read_u32())
        .await
        .map_err(|_| LinkError::Deadline)?
        .map_err(|_| LinkError::Frame)?;
    let length = usize::try_from(length).map_err(|_| LinkError::Frame)?;
    if length == 0 || length > MAX_HANDSHAKE_BYTES {
        return Err(LinkError::Frame);
    }
    let mut bytes = vec![0; length];
    timeout_at(deadline, stream.read_exact(&mut bytes))
        .await
        .map_err(|_| LinkError::Deadline)?
        .map_err(|_| LinkError::Frame)?;
    decode(&bytes).map_err(|_| LinkError::Frame)
}

pub(crate) async fn write<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
    deadline: Instant,
) -> Result<(), LinkError> {
    let bytes = encode(value).map_err(|_| LinkError::Frame)?;
    if bytes.is_empty() || bytes.len() > MAX_HANDSHAKE_BYTES {
        return Err(LinkError::Frame);
    }
    let length = u32::try_from(bytes.len()).map_err(|_| LinkError::Frame)?;
    timeout_at(deadline, async {
        stream.write_u32(length).await?;
        stream.write_all(&bytes).await?;
        stream.flush().await
    })
    .await
    .map_err(|_| LinkError::Deadline)?
    .map_err(|_| LinkError::Frame)
}
