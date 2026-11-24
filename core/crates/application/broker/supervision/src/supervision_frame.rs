use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::io::{Read, Write};
use uuid::Uuid;
use yolo_local_unix::BlockingLocalUnixStream;

pub(super) const MAX_FRAME_BYTES: usize = 16 * 1024;
const HEADER_BYTES: usize = std::mem::size_of::<u32>();
const FRAME_CONTEXT: &str = "exchange broker supervision Cap'n Proto frame";
const FRAME_LIMIT: &str = "broker supervision frame exceeds byte limit";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Control {
    Activate,
    Adopt { nonce: Uuid },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AdoptionPhase {
    Prepared,
    Commit,
    Committed,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Adoption {
    pub phase: AdoptionPhase,
    pub nonce: Uuid,
    pub instance_id: Uuid,
}

pub(super) fn read<T: DeserializeOwned>(stream: &mut BlockingLocalUnixStream) -> Result<T> {
    let mut header = [u8::default(); HEADER_BYTES];
    stream.read_exact(&mut header).context(FRAME_CONTEXT)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!(length <= MAX_FRAME_BYTES, FRAME_LIMIT);
    let mut frame = vec![u8::default(); length];
    stream.read_exact(&mut frame).context(FRAME_CONTEXT)?;
    yolo_capnp_codec::decode(&frame).context(FRAME_CONTEXT)
}

pub(super) fn write<T: Serialize>(stream: &mut BlockingLocalUnixStream, value: &T) -> Result<()> {
    let frame = yolo_capnp_codec::encode(value).context(FRAME_CONTEXT)?;
    ensure!(frame.len() <= MAX_FRAME_BYTES, FRAME_LIMIT);
    stream
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .context(FRAME_CONTEXT)?;
    stream.write_all(&frame).context(FRAME_CONTEXT)
}
