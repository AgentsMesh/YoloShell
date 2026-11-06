use crate::constants::CONTROL_PEER_UID_ERROR;
use anyhow::{Result, ensure};
use yolo_local_unix::LocalUnixStream;

pub(crate) fn verify(stream: &LocalUnixStream) -> Result<()> {
    let peer = stream.peer_cred()?;
    // SAFETY: geteuid only reads process credentials.
    ensure!(
        peer.uid() == unsafe { libc::geteuid() },
        CONTROL_PEER_UID_ERROR
    );
    Ok(())
}
