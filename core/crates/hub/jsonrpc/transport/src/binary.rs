use crate::{Framed, TransportError};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};

const BINARY_TAG: u8 = 1;
const HEADER_BYTES: usize = 5;
const LENGTH_OFFSET: usize = 1;

impl<S: AsyncWrite + Unpin> Framed<S> {
    /// Only negotiated terminal streams use this frame, immediately after its
    /// JSON metadata. The metadata length and binary length must agree exactly.
    pub async fn send_binary(
        &mut self,
        bytes: &[u8],
        limit: usize,
        deadline: Instant,
    ) -> Result<(), TransportError> {
        if bytes.len() > limit {
            return Err(TransportError::FrameTooLarge);
        }
        let length = u32::try_from(bytes.len()).map_err(|_| TransportError::FrameTooLarge)?;
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline);
        }
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        let mut header = [0; HEADER_BYTES];
        header[0] = BINARY_TAG;
        header[LENGTH_OFFSET..].copy_from_slice(&length.to_be_bytes());
        timeout_at(deadline, async {
            stream.write_all(&header).await?;
            stream.write_all(bytes).await?;
            stream.flush().await
        })
        .await
        .map_err(|_| TransportError::Deadline)??;
        self.stream = Some(stream);
        Ok(())
    }
}

impl<S: AsyncRead + Unpin> Framed<S> {
    pub async fn receive_binary(
        &mut self,
        expected: usize,
        limit: usize,
        deadline: Instant,
    ) -> Result<Vec<u8>, TransportError> {
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline);
        }
        let mut stream = self.stream.take().ok_or(TransportError::Poisoned)?;
        if expected > limit {
            return Err(TransportError::FrameTooLarge);
        }
        let bytes = timeout_at(deadline, async {
            let mut header = [0; HEADER_BYTES];
            read(&mut stream, &mut self.pending, &mut header).await?;
            let length = u32::from_be_bytes(header[LENGTH_OFFSET..].try_into().unwrap()) as usize;
            if header[0] != BINARY_TAG || length != expected {
                return Err(TransportError::InvalidBinary);
            }
            let mut bytes = vec![0; length];
            read(&mut stream, &mut self.pending, &mut bytes).await?;
            Ok::<_, TransportError>(bytes)
        })
        .await
        .map_err(|_| TransportError::Deadline)??;
        self.stream = Some(stream);
        Ok(bytes)
    }
}

async fn read<S: AsyncRead + Unpin>(
    stream: &mut S,
    pending: &mut Vec<u8>,
    output: &mut [u8],
) -> Result<(), TransportError> {
    let copied = pending.len().min(output.len());
    output[..copied].copy_from_slice(&pending[..copied]);
    pending.drain(..copied);
    stream
        .read_exact(&mut output[copied..])
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                TransportError::TruncatedFrame
            } else {
                TransportError::Io(error)
            }
        })?;
    Ok(())
}
