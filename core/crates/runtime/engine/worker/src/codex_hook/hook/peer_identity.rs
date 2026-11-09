use super::constants::{CODEX_PEER_PID_ERROR, CODEX_PEER_UID_ERROR};
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result};

pub(super) fn verify(stream: &LocalUnixStream) -> Result<crate::process::ProcessIdentity> {
    crate::worker::local_transport::verify_same_uid(stream).context(CODEX_PEER_UID_ERROR)?;
    let pid = stream
        .peer_cred()
        .context(CODEX_PEER_PID_ERROR)?
        .pid()
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value >= crate::agent::constants::MIN_PROCESS_PID)
        .context(CODEX_PEER_PID_ERROR)?;
    let identity = crate::process::ProcessIdentity::capture(pid).context(CODEX_PEER_PID_ERROR)?;
    anyhow::ensure!(identity.start_time != u64::default(), CODEX_PEER_PID_ERROR);
    Ok(identity)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/hook/peer_identity_inline_tests.rs"]
mod tests;
