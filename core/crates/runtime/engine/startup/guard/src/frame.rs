use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Commit,
    Committed,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Frame {
    pub version: u16,
    pub process_id: u32,
    pub phase: Phase,
}

impl Frame {
    pub fn new(process_id: u32, phase: Phase) -> Self {
        Self {
            version: VERSION,
            process_id,
            phase,
        }
    }
    pub fn validate(&self, process_id: u32, phase: Phase) -> Result<()> {
        ensure!(
            self.version == VERSION
                && process_id != 0
                && self.process_id == process_id
                && self.phase == phase,
            INVALID_FRAME
        );
        Ok(())
    }
}

pub(crate) async fn read(stream: &mut UnixStream) -> Result<Frame> {
    let length = stream.read_u32().await? as usize;
    ensure!(length > 0 && length <= MAX_FRAME, INVALID_FRAME);
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    yolo_capnp_codec::decode(&bytes).map_err(|_| anyhow::anyhow!(INVALID_FRAME))
}

pub(crate) async fn write(stream: &mut UnixStream, frame: Frame) -> Result<()> {
    let bytes = encode(frame)?;
    stream.write_all(&bytes).await?;
    Ok(())
}

pub(crate) fn encode(frame: Frame) -> Result<Vec<u8>> {
    let payload = yolo_capnp_codec::encode(&frame)?;
    ensure!(payload.len() <= MAX_FRAME, INVALID_FRAME);
    let mut bytes = (payload.len() as u32).to_be_bytes().to_vec();
    bytes.extend(payload);
    Ok(bytes)
}
