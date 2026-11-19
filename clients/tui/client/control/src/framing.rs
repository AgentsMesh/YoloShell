use crate::constants::*;
use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};

pub(crate) async fn receive<S: AsyncRead + Unpin, T: DeserializeOwned>(
    stream: &mut S,
    deadline: Instant,
) -> Result<T> {
    ensure!(Instant::now() < deadline, CONTROL_DEADLINE);
    timeout_at(deadline, async {
        let mut header = [u8::default(); CONTROL_FRAME_LENGTH_BYTES];
        stream.read_exact(&mut header).await?;
        let length = u32::from_be_bytes(header) as usize;
        ensure!(
            length > usize::default() && length <= MAX_CONTROL_FRAME_BYTES,
            CONTROL_PROTOCOL_ERROR
        );
        let mut payload = vec![u8::default(); length];
        stream.read_exact(&mut payload).await?;
        Ok(yolo_capnp_codec::decode(&payload)?)
    })
    .await
    .context(CONTROL_DEADLINE)?
}

pub(crate) async fn send<S: AsyncWrite + Unpin, T: Serialize>(
    stream: &mut S,
    value: &T,
    deadline: Instant,
) -> Result<()> {
    ensure!(Instant::now() < deadline, CONTROL_DEADLINE);
    let payload = yolo_capnp_codec::encode(value)?;
    ensure!(
        !payload.is_empty() && payload.len() <= MAX_CONTROL_FRAME_BYTES,
        CONTROL_PROTOCOL_ERROR
    );
    timeout_at(deadline, async {
        stream
            .write_all(&u32::try_from(payload.len())?.to_be_bytes())
            .await?;
        stream.write_all(&payload).await?;
        stream.flush().await?;
        Ok(())
    })
    .await
    .context(CONTROL_DEADLINE)?
}
