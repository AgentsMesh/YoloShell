use super::limits::{MAX_PROCESS_SNAPSHOT_FRAME_BYTES, MAX_PROCESS_SNAPSHOT_NESTING_DEPTH};
use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt};

const FRAME_HEADER_CONTEXT: &str = "read process snapshot frame header";
const FRAME_PAYLOAD_CONTEXT: &str = "read process snapshot frame payload";
const FRAME_SIZE_INVALID: &str = "process snapshot frame exceeds its byte budget";
const CAPNP_DECODE_CONTEXT: &str = "decode process snapshot Cap'n Proto document";

pub(super) async fn read_payload(reader: &mut (impl AsyncRead + Unpin)) -> Result<Vec<u8>> {
    let mut header = [u8::default(); crate::protocol::FRAME_LENGTH_BYTES];
    reader
        .read_exact(&mut header)
        .await
        .context(FRAME_HEADER_CONTEXT)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(
        length <= MAX_PROCESS_SNAPSHOT_FRAME_BYTES,
        FRAME_SIZE_INVALID
    );
    let mut payload = vec![u8::default(); length];
    reader
        .read_exact(&mut payload)
        .await
        .context(FRAME_PAYLOAD_CONTEXT)?;
    Ok(payload)
}

pub(super) fn decode_strict<T: DeserializeOwned>(payload: &[u8]) -> Result<T> {
    crate::protocol::decode_with_depth(payload, MAX_PROCESS_SNAPSHOT_NESTING_DEPTH).context(CAPNP_DECODE_CONTEXT)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/process_snapshot_decode_tests.rs"]
mod tests;
