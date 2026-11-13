use crate::{MAX_READY_BYTES, ReadyDocument};
use anyhow::{Result, ensure};
use tokio::io::{AsyncRead, AsyncReadExt};

const HEADER_BYTES: usize = std::mem::size_of::<u32>();
const INVALID_READY: &str = "invalid bounded Hub readiness frame";

pub fn encode_ready(document: &ReadyDocument) -> Result<Vec<u8>> {
    let payload = yolo_capnp_codec::encode(document)?;
    ensure!(payload.len() <= MAX_READY_BYTES, INVALID_READY);
    let mut frame = u32::try_from(payload.len())?.to_be_bytes().to_vec();
    frame.extend(payload);
    Ok(frame)
}

pub(crate) async fn read_ready(
    reader: &mut (impl AsyncRead + Unpin),
) -> Result<Option<ReadyDocument>> {
    let mut header = [0; HEADER_BYTES];
    if reader.read(&mut header[..1]).await? == 0 {
        return Ok(None);
    }
    reader.read_exact(&mut header[1..]).await?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(length > 0 && length <= MAX_READY_BYTES, INVALID_READY);
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).await?;
    Ok(Some(yolo_capnp_codec::decode(&payload)?))
}
