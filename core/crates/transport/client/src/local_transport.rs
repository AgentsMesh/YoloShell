use super::ClientError;
use crate::local_unix::LocalUnixStream;
use crate::protocol::ProtocolError;

const HOST_PEER_CREDENTIAL_CONTEXT: &str = "read local host peer credentials";
const HOST_PEER_UID_DETAIL: &str = "local host peer effective uid does not match the client";

pub(super) fn verify_peer_uid(stream: &LocalUnixStream) -> Result<(), ClientError> {
    let peer = stream.peer_cred().map_err(|error| {
        ProtocolError::Io(std::io::Error::new(
            error.kind(),
            format!("{HOST_PEER_CREDENTIAL_CONTEXT}: {error}"),
        ))
    })?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    validate_peer_uid(peer.uid(), unsafe { libc::geteuid() })
}

fn validate_peer_uid(peer: libc::uid_t, expected: libc::uid_t) -> Result<(), ClientError> {
    if peer == expected {
        return Ok(());
    }
    Err(ProtocolError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        HOST_PEER_UID_DETAIL,
    ))
    .into())
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/local_transport_inline_tests.rs"]
mod tests;
