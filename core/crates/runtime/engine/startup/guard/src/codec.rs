use crate::{
    constants::*,
    frame::{Frame, Phase},
};
use anyhow::{Result, ensure};

const LENGTH_BYTES: usize = std::mem::size_of::<u32>();

/// Complete bounded binary frame for a direct-child startup capability.
pub fn encode_commit(process_id: u32) -> Result<Vec<u8>> {
    let frame = Frame::new(process_id, Phase::Commit);
    frame.validate(process_id, Phase::Commit)?;
    crate::frame::encode(frame)
}

/// Validate a complete response frame, including its length prefix and exact child identity.
pub fn validate_committed(bytes: &[u8], process_id: u32) -> Result<()> {
    ensure!(
        bytes.len() >= LENGTH_BYTES && bytes.len() <= MAX_FRAME + LENGTH_BYTES,
        INVALID_FRAME
    );
    let length = u32::from_be_bytes(bytes[..LENGTH_BYTES].try_into()?) as usize;
    ensure!(length == bytes.len() - LENGTH_BYTES, INVALID_FRAME);
    let frame: Frame = yolo_capnp_codec::decode(&bytes[LENGTH_BYTES..])?;
    frame.validate(process_id, Phase::Committed)
}
