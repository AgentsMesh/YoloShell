//! Hidden stdio-to-peer-socket bridge used by the SSH peer transport.
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::{copy_bidirectional, join};
use tokio::net::UnixStream;

const CONNECT_FAILED: &str = "connect the SSH peer stream to the local peer socket";

pub(crate) async fn run(socket: PathBuf) -> Result<()> {
    let mut stream = UnixStream::connect(&socket)
        .await
        .context(CONNECT_FAILED)?;
    let mut terminal = join(tokio::io::stdin(), tokio::io::stdout());
    copy_bidirectional(&mut terminal, &mut stream).await?;
    Ok(())
}
