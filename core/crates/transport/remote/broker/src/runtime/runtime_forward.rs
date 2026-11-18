use super::constants::*;
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(super) async fn forward(
    local: LocalUnixStream,
    remote: crate::remote::BoxedRemoteChannel,
    local_prefix: Vec<u8>,
) -> Result<()> {
    let (mut local_read, mut local_write) = tokio::io::split(local);
    let (mut remote_read, mut remote_write) = tokio::io::split(remote);
    let mut local_to_remote = Box::pin(pump(
        &mut local_read,
        &mut remote_write,
        local_prefix,
        RUNTIME_LOCAL_WRITE_TIMEOUT_MESSAGE,
    ));
    let mut remote_to_local = Box::pin(pump(
        &mut remote_read,
        &mut local_write,
        Vec::new(),
        RUNTIME_REMOTE_WRITE_TIMEOUT_MESSAGE,
    ));
    tokio::select! {
        result = &mut local_to_remote => {
            drop(local_to_remote);
            result?;
            let closed = shutdown(
                &mut remote_write,
                RUNTIME_REMOTE_SHUTDOWN_TIMEOUT,
                RUNTIME_REMOTE_SHUTDOWN_TIMEOUT_MESSAGE,
            ).await;
            let drained = drain(&mut remote_to_local, RUNTIME_REMOTE_TAIL_TIMEOUT_MESSAGE).await;
            drop(remote_to_local);
            let local_closed = shutdown_local(&mut local_write).await;
            drained?;
            local_closed?;
            closed
        }
        result = &mut remote_to_local => {
            drop(remote_to_local);
            result?;
            let closed = shutdown_local(&mut local_write).await;
            let drained = drain(&mut local_to_remote, RUNTIME_LOCAL_TAIL_TIMEOUT_MESSAGE).await;
            drop(local_to_remote);
            let remote_closed = shutdown(
                &mut remote_write,
                RUNTIME_REMOTE_SHUTDOWN_TIMEOUT,
                RUNTIME_REMOTE_SHUTDOWN_TIMEOUT_MESSAGE,
            ).await;
            drained?;
            remote_closed?;
            closed
        }
    }
}

async fn pump<R, W>(
    reader: &mut R,
    writer: &mut W,
    prefix: Vec<u8>,
    message: &'static str,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if !prefix.is_empty() {
        write(writer, &prefix, message).await?;
    }
    let mut buffer = [0_u8; RUNTIME_FORWARD_BUFFER_BYTES];
    loop {
        let bytes = reader.read(&mut buffer).await?;
        if bytes == 0 {
            return Ok(());
        }
        write(writer, &buffer[..bytes], message).await?;
    }
}

async fn write<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
    message: &'static str,
) -> Result<()> {
    tokio::time::timeout(RUNTIME_FORWARD_WRITE_TIMEOUT, writer.write_all(bytes))
        .await
        .map_err(|_| anyhow::anyhow!(message))??;
    Ok(())
}

async fn drain<F>(pump: &mut Pin<Box<F>>, message: &'static str) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    tokio::time::timeout(RUNTIME_TAIL_DRAIN_TIMEOUT, pump)
        .await
        .map_err(|_| anyhow::anyhow!(message))??;
    Ok(())
}

pub(super) async fn shutdown<W: AsyncWrite + Unpin>(
    writer: &mut W,
    timeout: std::time::Duration,
    message: &'static str,
) -> Result<()> {
    tokio::time::timeout(timeout, writer.shutdown())
        .await
        .map_err(|_| anyhow::anyhow!(message))??;
    Ok(())
}

pub(super) async fn shutdown_local<W: AsyncWrite + Unpin>(writer: &mut W) -> Result<()> {
    match tokio::time::timeout(RUNTIME_LOCAL_SHUTDOWN_TIMEOUT, writer.shutdown()).await {
        Err(_) => Err(anyhow::anyhow!(RUNTIME_LOCAL_SHUTDOWN_TIMEOUT_MESSAGE)),
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) if local_shutdown_is_complete(error.kind()) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
    }
}

fn local_shutdown_is_complete(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::NotConnected
    )
}
