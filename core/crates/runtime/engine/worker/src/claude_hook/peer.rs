use super::constants::{PEER_PID_ERROR, PEER_UID_ERROR};
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result, ensure};

pub(super) fn pid(stream: &LocalUnixStream) -> Result<u32> {
    let credentials = stream.peer_cred().context(PEER_PID_ERROR)?;
    // SAFETY: geteuid only reads the calling process credential.
    ensure!(
        credentials.uid() == unsafe { libc::geteuid() },
        PEER_UID_ERROR
    );
    let pid = credentials.pid().context(PEER_PID_ERROR)?;
    ensure!(pid > 0, PEER_PID_ERROR);
    Ok(pid as u32)
}
