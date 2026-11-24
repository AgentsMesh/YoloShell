use super::*;

const PEER_IDENTITY_DETAIL: &str = "PTY custody socket peer is not the exact Worker process";

pub(super) fn validate(
    _host: &Host,
    stream: &CustodyUnixStream,
    record: &RuntimeRecord,
) -> Result<(), WorkerRpcError> {
    let peer_pid = stream.peer_pid().map_err(identity_error)?;
    let peer = crate::process::ProcessIdentity::capture(peer_pid).map_err(identity_error)?;
    #[cfg(test)]
    if _host.secondary_custody.peer_test_authorized(record, peer) {
        return Ok(());
    }
    (peer.pid == record.worker_pid && peer.start_time == record.worker_start_time)
        .then_some(())
        .ok_or_else(|| WorkerRpcError::identity_mismatch(PEER_IDENTITY_DETAIL))
}

fn identity_error(error: anyhow::Error) -> WorkerRpcError {
    WorkerRpcError::identity_mismatch(super::containment::error_detail(error))
}
