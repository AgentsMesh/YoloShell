use anyhow::{Context, Result, ensure};
use yolo_local_unix::LocalUnixStream;

const PEER_UID: &str = "provider hook peer user does not match this Worker";
const PEER_PID: &str = "provider hook peer process identity is unavailable";

pub(super) fn pid(stream: &LocalUnixStream) -> Result<u32> {
    let peer = stream.peer_cred()?;
    // SAFETY: geteuid has no preconditions.
    ensure!(peer.uid() == unsafe { libc::geteuid() }, PEER_UID);
    let pid = peer.pid().context(PEER_PID)?;
    u32::try_from(pid).context(PEER_PID)
}
