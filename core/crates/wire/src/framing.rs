use super::{FRAME_LENGTH_BYTES, MAX_FRAME_BYTES, ProtocolError, decode, encode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Eq, PartialEq)]
pub enum OptionalFrameRead<T> {
    CleanEof,
    Frame(T),
}

pub async fn read_optional_frame<R, T>(
    reader: &mut R,
) -> Result<OptionalFrameRead<T>, ProtocolError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut length = [0_u8; FRAME_LENGTH_BYTES];
    let mut received = 0;
    while received < FRAME_LENGTH_BYTES {
        match reader.read(&mut length[received..]).await {
            Ok(0) if received == 0 => return Ok(OptionalFrameRead::CleanEof),
            Ok(0) => return Err(ProtocolError::TruncatedHeader { received }),
            Ok(count) => received += count,
            Err(error) => return Err(ProtocolError::Io(error)),
        }
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge { length });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(ProtocolError::Io)?;
    decode(&payload).map(OptionalFrameRead::Frame)
}

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
    if length > MAX_FRAME_BYTES {
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
    writer
        .write_all(&encode(value)?)
        .await
        .map_err(ProtocolError::Io)
}

pub fn read_frame_blocking<R, T>(reader: &mut R) -> Result<T, ProtocolError>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut length = [0_u8; FRAME_LENGTH_BYTES];
    if let Err(error) = reader.read_exact(&mut length) {
        return if error.kind() == std::io::ErrorKind::UnexpectedEof {
            Err(ProtocolError::EndOfStream)
        } else {
            Err(ProtocolError::Io(error))
        };
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge { length });
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).map_err(ProtocolError::Io)?;
    decode(&payload)
}

pub fn write_frame_blocking<W, T>(writer: &mut W, value: &T) -> Result<(), ProtocolError>
where
    W: Write,
    T: Serialize,
{
    writer.write_all(&encode(value)?).map_err(ProtocolError::Io)
}
