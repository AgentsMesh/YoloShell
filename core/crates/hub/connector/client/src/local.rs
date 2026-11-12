use crate::{Client, ClientError};
use std::path::{Path, PathBuf};
use tokio::time::{Instant, timeout_at};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_local_unix::LocalUnixStream;

/// Resolves the companion endpoint inside the Hub's already-discovered short namespace.
pub fn socket_for_hub(hub_socket: &Path) -> Result<PathBuf, ClientError> {
    if !hub_socket.is_absolute() || hub_socket.file_name().is_none() {
        return Err(ClientError::Validation(
            yolo_hub_connector_protocol::ConnectorError::InvalidRequest,
        ));
    }
    let parent = hub_socket.parent().ok_or(ClientError::Validation(
        yolo_hub_connector_protocol::ConnectorError::InvalidRequest,
    ))?;
    Ok(parent.join(yolo_hub_connector_protocol::CONNECTOR_SOCKET_FILE))
}

impl Client<LocalUnixStream> {
    pub async fn connect_local(
        path: &Path,
        expected: HubId,
        deadline: Instant,
    ) -> Result<Self, ClientError> {
        if Instant::now() >= deadline {
            return Err(TransportError::Deadline.into());
        }
        let stream = timeout_at(deadline, LocalUnixStream::connect(path))
            .await
            .map_err(|_| TransportError::Deadline)?
            .map_err(TransportError::Io)?;
        let peer = stream.peer_cred().map_err(TransportError::Io)?;
        validate_uid(peer.uid(), unsafe { libc::geteuid() })?;
        Self::open(stream, expected, deadline).await
    }
}

pub(crate) fn validate_uid(actual: u32, expected: u32) -> Result<(), ClientError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ClientError::WrongLocalUser)
    }
}
