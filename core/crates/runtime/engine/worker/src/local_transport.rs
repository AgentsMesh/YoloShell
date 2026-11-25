use super::WorkerRpcError;
use crate::local_unix::LocalUnixStream;
use crate::model::RuntimeRecord;
use crate::process::ProcessIdentity;
use anyhow::{Context, Result, ensure};

const PEER_CREDENTIAL_CONTEXT: &str = "read local worker peer credentials";
const PEER_UID_DETAIL: &str = "local worker peer effective uid does not match this process";
const SERVER_CREDENTIAL_DETAIL: &str = "local worker server credentials are unavailable";
const SERVER_UID_DETAIL: &str = "local worker server effective uid does not match the client";
const SERVER_IDENTITY_DETAIL: &str = "local worker server process identity does not match runtime";

pub(super) fn verify_same_uid(stream: &LocalUnixStream) -> Result<()> {
    let peer = stream.peer_cred().context(PEER_CREDENTIAL_CONTEXT)?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    ensure!(peer.uid() == unsafe { libc::geteuid() }, PEER_UID_DETAIL);
    Ok(())
}

pub(super) fn verify_server(
    stream: &LocalUnixStream,
    expected: &RuntimeRecord,
) -> std::result::Result<(), WorkerRpcError> {
    verify_server_identity(
        stream,
        ProcessIdentity {
            pid: expected.worker_pid,
            start_time: expected.worker_start_time,
        },
    )
}

pub(super) fn verify_server_identity(
    stream: &LocalUnixStream,
    expected: ProcessIdentity,
) -> std::result::Result<(), WorkerRpcError> {
    let peer = stream
        .peer_cred()
        .map_err(|error| WorkerRpcError::identity_mismatch(error.to_string()))?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    if peer.uid() != unsafe { libc::geteuid() } {
        return Err(WorkerRpcError::identity_mismatch(SERVER_UID_DETAIL));
    }
    let pid = peer
        .pid()
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid != u32::default())
        .ok_or_else(|| WorkerRpcError::identity_mismatch(SERVER_CREDENTIAL_DETAIL))?;
    let identity = ProcessIdentity::capture(pid)
        .map_err(|error| WorkerRpcError::identity_mismatch(error.to_string()))?;
    if identity != expected {
        return Err(WorkerRpcError::identity_mismatch(SERVER_IDENTITY_DETAIL));
    }
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/local_transport_inline_tests.rs"]
mod tests;
