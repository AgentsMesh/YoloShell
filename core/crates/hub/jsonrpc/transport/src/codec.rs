use crate::{FRAME_DELIMITER, MAX_FRAME_BYTES, TransportError};
use serde::Serialize;
use std::io::{self, Write};

struct Limited {
    bytes: Vec<u8>,
    exceeded: bool,
}
impl Write for Limited {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.len() > MAX_FRAME_BYTES.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("JSON-RPC frame too large"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, TransportError> {
    let mut writer = Limited {
        bytes: Vec::new(),
        exceeded: false,
    };
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        return Err(if writer.exceeded {
            TransportError::FrameTooLarge
        } else {
            TransportError::Encode(error)
        });
    }
    writer.bytes.push(FRAME_DELIMITER);
    Ok(writer.bytes)
}
