use crate::constants::CLEAN_EOF_BYTES;
use crate::{FRAME_LENGTH_BYTES, MAX_FRAME_BYTES, TransportError, decode, encode};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Instant, timeout_at};

#[derive(Debug, Eq, PartialEq)]
pub enum Incoming<T> {
    Message(T),
    Eof,
}

/// Taking the stream before an await makes cancellation close it, rather than
/// letting a subsequent request consume or complete a partially transferred frame.
pub struct Framed<S> {
    stream: Option<S>,
}

impl<S> Framed<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
        }
    }
    pub fn is_usable(&self) -> bool {
        self.stream.is_some()
    }

    pub fn into_stream(self) -> Result<S, TransportError> {
        self.stream.ok_or(TransportError::Poisoned)
    }
}

impl<S: AsyncWrite + Unpin> Framed<S> {
    pub async fn send<T: Serialize>(
        &mut self,
        value: &T,
        deadline: Instant,
    ) -> Result<(), TransportError> {
        let bytes = encode(value)?;
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline);
        }
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        timeout_at(deadline, async {
            stream.write_all(&bytes).await?;
            stream.flush().await
        })
        .await
        .map_err(|_| TransportError::Deadline)??;
        self.stream = Some(stream);
        Ok(())
    }

    pub async fn close(&mut self, deadline: Instant) -> Result<(), TransportError> {
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        timeout_at(deadline, stream.shutdown())
            .await
            .map_err(|_| TransportError::Deadline)??;
        Ok(())
    }
}

impl<S: AsyncRead + Unpin> Framed<S> {
    pub async fn receive<T: DeserializeOwned>(
        &mut self,
        deadline: Instant,
    ) -> Result<Incoming<T>, TransportError> {
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline);
        }
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        let result = timeout_at(deadline, receive(&mut stream))
            .await
            .map_err(|_| TransportError::Deadline)??;
        if matches!(result, Incoming::Message(_)) {
            self.stream = Some(stream);
        }
        Ok(result)
    }
}

async fn receive<S: AsyncRead + Unpin, T: DeserializeOwned>(
    stream: &mut S,
) -> Result<Incoming<T>, TransportError> {
    let mut header = [u8::default(); FRAME_LENGTH_BYTES];
    let mut received = usize::default();
    while received < header.len() {
        let read = stream.read(&mut header[received..]).await?;
        if read == CLEAN_EOF_BYTES {
            return if received == CLEAN_EOF_BYTES {
                Ok(Incoming::Eof)
            } else {
                Err(TransportError::TruncatedFrame)
            };
        }
        received += read;
    }
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(TransportError::FrameTooLarge);
    }
    if length == CLEAN_EOF_BYTES {
        return Err(TransportError::EmptyFrame);
    }
    let mut payload = vec![u8::default(); length];
    stream.read_exact(&mut payload).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            TransportError::TruncatedFrame
        } else {
            TransportError::Io(error)
        }
    })?;
    decode(&payload).map(Incoming::Message)
}
