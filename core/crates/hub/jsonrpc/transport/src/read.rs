use crate::{FRAME_DELIMITER, Incoming, MAX_FRAME_BYTES, TransportError};
use tokio::io::{AsyncRead, AsyncReadExt};

const READ_BUFFER_BYTES: usize = 8 * 1_024;

pub(crate) async fn receive<S: AsyncRead + Unpin>(
    stream: &mut S,
    pending: &mut Vec<u8>,
) -> Result<Incoming<Vec<u8>>, TransportError> {
    let mut scanned = 0;
    loop {
        if let Some(offset) = pending[scanned..]
            .iter()
            .position(|byte| *byte == FRAME_DELIMITER)
        {
            let end = scanned + offset;
            if end > MAX_FRAME_BYTES {
                return Err(TransportError::FrameTooLarge);
            }
            let remainder = pending.split_off(end + 1);
            let mut frame = std::mem::replace(pending, remainder);
            frame.pop();
            if frame.is_empty() {
                return Err(TransportError::EmptyFrame);
            }
            return Ok(Incoming::Message(frame));
        }
        if pending.len() > MAX_FRAME_BYTES {
            return Err(TransportError::FrameTooLarge);
        }
        scanned = pending.len();
        let mut bytes = [0; READ_BUFFER_BYTES];
        let count = stream.read(&mut bytes).await?;
        if count == 0 {
            return if pending.is_empty() {
                Ok(Incoming::Eof)
            } else {
                Err(TransportError::TruncatedFrame)
            };
        }
        pending.extend_from_slice(&bytes[..count]);
    }
}
