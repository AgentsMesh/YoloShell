use crate::{INPUT_QUEUE_CAPACITY, PING_QUEUE_CAPACITY, STREAM_CHUNK_BYTES};
use anyhow::{Result, bail};
use futures::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

/// Binary messages are slices of the unchanged public Hub byte stream. They
/// may split/coalesce JSON lines and terminal blobs; no authority is synthesized.
pub(crate) async fn bridge<S: AsyncRead + AsyncWrite + Unpin>(
    web: &mut WebSocketStream<TcpStream>,
    hub: &mut S,
) -> Result<()> {
    let (mut sink, mut source) = web.split();
    let (mut reader, mut writer) = tokio::io::split(hub);
    let (ping, mut pong) = mpsc::channel(PING_QUEUE_CAPACITY);
    let (input, mut queued) = mpsc::channel(INPUT_QUEUE_CAPACITY);
    let inbound = async {
        loop {
            match source.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    // Never stop reading Close behind a blocked Hub write. A full
                    // bounded queue retires this channel; an uncertain prefix is
                    // neither acknowledged nor automatically replayed.
                    input.try_send(bytes).map_err(|_| {
                        anyhow::anyhow!("web input queue exhausted; channel retired")
                    })?;
                    // A buffered burst must give the writer a chance to drain.
                    tokio::task::yield_now().await;
                }
                Some(Ok(Message::Ping(_))) => {
                    let _ = ping.try_send(());
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Err(error)) => return Err(error.into()),
                _ => bail!("web Hub stream requires binary messages"),
            }
        }
    };
    let forwarding = async {
        while let Some(bytes) = queued.recv().await {
            writer.write_all(&bytes).await?;
        }
        Ok(())
    };
    let outbound = async {
        let mut buffer = vec![0; STREAM_CHUNK_BYTES];
        loop {
            tokio::select! {
                _ = pong.recv() => sink.flush().await?,
                count = reader.read(&mut buffer) => {
                    let count = count?;
                    if count == 0 { return Ok(()); }
                    sink.send(Message::Binary(buffer[..count].to_vec().into())).await?;
                }
            }
        }
    };
    tokio::select! {
        result = inbound => result,
        result = forwarding => result,
        result = outbound => result,
    }
}
