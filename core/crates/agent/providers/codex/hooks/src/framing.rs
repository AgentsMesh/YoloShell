use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) async fn read_frame<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
    maximum: usize,
    context: &'static str,
) -> Result<T> {
    let mut header = [u8::default(); super::constants::FRAME_HEADER_BYTES];
    reader.read_exact(&mut header).await.context(context)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(length <= maximum, context);
    let mut payload = vec![u8::default(); length];
    reader.read_exact(&mut payload).await.context(context)?;
    yolo_capnp_codec::decode(&payload).context(context)
}

pub(super) async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
    maximum: usize,
    context: &'static str,
) -> Result<()> {
    let payload = yolo_capnp_codec::encode(value).context(context)?;
    ensure!(payload.len() <= maximum, context);
    let length = u32::try_from(payload.len()).context(context)?;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .context(context)?;
    writer.write_all(&payload).await.context(context)
}
