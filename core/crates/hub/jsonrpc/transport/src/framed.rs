use crate::{Buffered, TransportError, encode, read};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::time::{Instant, timeout_at};

#[derive(Debug, Eq, PartialEq)]
pub enum Incoming<T> {
    Message(T),
    Eof,
}

/// Cancellation owns and closes the stream until a complete frame is transferred.
pub struct Framed<S> {
    pub(crate) stream: Option<S>,
    pub(crate) pending: Vec<u8>,
}

impl<S> Framed<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
            pending: Vec::new(),
        }
    }
    pub fn is_usable(&self) -> bool {
        self.stream.is_some()
    }
    pub fn retire(&mut self) {
        self.stream.take();
        self.pending.clear();
    }
    pub fn into_stream(self) -> Result<Buffered<S>, TransportError> {
        Ok(Buffered {
            stream: self.stream.ok_or(TransportError::Poisoned)?,
            pending: self.pending,
        })
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
    pub async fn receive_frame(
        &mut self,
        deadline: Instant,
    ) -> Result<Incoming<Vec<u8>>, TransportError> {
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline);
        }
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        let result = timeout_at(deadline, read::receive(&mut stream, &mut self.pending))
            .await
            .map_err(|_| TransportError::Deadline)??;
        if matches!(result, Incoming::Message(_)) {
            self.stream = Some(stream);
        }
        Ok(result)
    }
    pub async fn receive<T: DeserializeOwned>(
        &mut self,
        deadline: Instant,
    ) -> Result<Incoming<T>, TransportError> {
        match self.receive_frame(deadline).await? {
            Incoming::Message(bytes) => match yolo_hub_jsonrpc_protocol::decode(&bytes) {
                Ok(value) => Ok(Incoming::Message(value)),
                Err(error) => {
                    self.stream.take();
                    Err(TransportError::Decode(error))
                }
            },
            Incoming::Eof => Ok(Incoming::Eof),
        }
    }
}
