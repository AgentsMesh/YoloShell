use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_local_unix::{BlockingLocalUnixStream, LocalUnixStream};
use zeroize::Zeroizing;
#[path = "frame_blocking.rs"]
mod blocking;

pub(crate) async fn read_async<T: DeserializeOwned>(
    stream: &mut LocalUnixStream,
    deadline: tokio::time::Instant,
) -> Result<T> {
    tokio::time::timeout_at(deadline, async {
        let length = stream.read_u32().await? as usize;
        ensure!(length <= MAX_FRAME_BYTES, INVALID);
        let mut bytes = Zeroizing::new(vec![u8::default(); length]);
        stream.read_exact(&mut bytes).await?;
        Ok(yolo_capnp_codec::decode(&bytes)?)
    })
    .await?
}
pub(crate) async fn write_async<T: Serialize>(
    stream: &mut LocalUnixStream,
    value: &T,
    deadline: tokio::time::Instant,
) -> Result<()> {
    let bytes = Zeroizing::new(yolo_capnp_codec::encode(value)?);
    write_bytes_async(stream, &bytes, deadline).await
}
pub(crate) async fn write_bytes_async(
    stream: &mut LocalUnixStream,
    bytes: &[u8],
    deadline: tokio::time::Instant,
) -> Result<()> {
    ensure!(bytes.len() <= MAX_FRAME_BYTES, INVALID);
    tokio::time::timeout_at(deadline, async {
        stream.write_u32(u32::try_from(bytes.len())?).await?;
        stream.write_all(bytes).await?;
        Ok(())
    })
    .await?
}
pub(crate) fn read<T: DeserializeOwned>(
    stream: &mut BlockingLocalUnixStream,
    deadline: tokio::time::Instant,
) -> Result<T> {
    Ok(yolo_capnp_codec::decode(&read_bytes(stream, deadline)?)?)
}
pub(crate) fn read_bytes(
    stream: &mut BlockingLocalUnixStream,
    deadline: tokio::time::Instant,
) -> Result<Zeroizing<Vec<u8>>> {
    blocking::prepare(stream, deadline)?;
    let mut header = [u8::default(); std::mem::size_of::<u32>()];
    blocking::read_exact(stream, &mut header, deadline)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(length <= MAX_FRAME_BYTES, INVALID);
    let mut bytes = Zeroizing::new(vec![u8::default(); length]);
    blocking::read_exact(stream, &mut bytes, deadline)?;
    Ok(bytes)
}
pub(crate) fn write<T: Serialize>(
    stream: &mut BlockingLocalUnixStream,
    value: &T,
    deadline: tokio::time::Instant,
) -> Result<()> {
    let bytes = Zeroizing::new(yolo_capnp_codec::encode(value)?);
    ensure!(bytes.len() <= MAX_FRAME_BYTES, INVALID);
    blocking::prepare(stream, deadline)?;
    blocking::write_all(stream, &u32::try_from(bytes.len())?.to_be_bytes(), deadline)?;
    blocking::write_all(stream, &bytes, deadline)
}
