use crate::protocol::{FRAME_LENGTH_BYTES, ProtocolError, decode, encode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn read_frame<R, T>(reader: &mut R) -> Result<T, ProtocolError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut length = [0_u8; FRAME_LENGTH_BYTES];
    match reader.read_exact(&mut length).await {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(ProtocolError::EndOfStream);
        }
        Err(error) => return Err(ProtocolError::Io(error)),
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > super::MAX_CLAUDE_HOOK_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge { length });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(ProtocolError::Io)?;
    decode(&payload)
}

pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<(), ProtocolError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let frame = encode(value)?;
    let payload_length = frame.len().saturating_sub(FRAME_LENGTH_BYTES);
    if payload_length > super::MAX_CLAUDE_HOOK_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: payload_length,
        });
    }
    writer.write_all(&frame).await.map_err(ProtocolError::Io)
}
