use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) const FRAME_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
pub(super) const MAXIMUM_FRAME_BYTES: usize = yolo_capnp_codec::MAX_MESSAGE_BYTES;

#[derive(Debug)]
pub(super) enum FrameError {
    EndOfStream,
    Io(std::io::Error),
    TooLarge { length: usize },
    Encode(yolo_capnp_codec::ValueError),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EndOfStream => write!(formatter, "catalog stream ended"),
            Self::Io(error) => write!(formatter, "catalog stream I/O error: {error}"),
            Self::TooLarge { length } => {
                write!(formatter, "catalog frame length {length} exceeds limit")
            }
            Self::Encode(error) => write!(formatter, "catalog frame encoding error: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::EndOfStream | Self::TooLarge { .. } => None,
        }
    }
}

impl From<std::io::Error> for FrameError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<yolo_capnp_codec::ValueError> for FrameError {
    fn from(error: yolo_capnp_codec::ValueError) -> Self {
        Self::Encode(error)
    }
}

pub(super) async fn read_catalog_frame<R, T>(reader: &mut R) -> Result<T, FrameError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut header = [u8::default(); FRAME_LENGTH_BYTES];
    match reader.read(&mut header[..1]).await {
        Ok(0) => return Err(FrameError::EndOfStream),
        Ok(_) => {}
        Err(error) => return Err(error.into()),
    }
    reader.read_exact(&mut header[1..]).await?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAXIMUM_FRAME_BYTES {
        return Err(FrameError::TooLarge { length });
    }
    let mut payload = vec![u8::default(); length];
    reader.read_exact(&mut payload).await?;
    Ok(yolo_capnp_codec::decode(&payload)?)
}

pub(super) async fn write_catalog_frame<W, T>(writer: &mut W, value: &T) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = yolo_capnp_codec::encode(value)?;
    if payload.len() > MAXIMUM_FRAME_BYTES {
        return Err(FrameError::TooLarge {
            length: payload.len(),
        });
    }
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}
