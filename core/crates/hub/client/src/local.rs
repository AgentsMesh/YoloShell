use crate::{Client, ClientError};
use std::path::Path;
use tokio::time::{Instant, timeout_at};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_local_unix::LocalUnixStream;

impl Client<LocalUnixStream> {
    pub async fn connect_local(
        path: &Path,
        expected: HubId,
        deadline: Instant,
    ) -> Result<Self, ClientError> {
        let stream = connect_local_stream(path, deadline).await?;
        Self::open(stream, expected, deadline).await
    }
}

pub(crate) async fn connect_local_stream(
    path: &Path,
    deadline: Instant,
) -> Result<LocalUnixStream, ClientError> {
    let stream = timeout_at(deadline, LocalUnixStream::connect(path))
        .await
        .map_err(|_| TransportError::Deadline)?
        .map_err(TransportError::Io)?;
    let credentials = stream.peer_cred().map_err(TransportError::Io)?;
    validate_uid(credentials.uid(), unsafe { libc::geteuid() })?;
    Ok(stream)
}

pub(crate) fn validate_uid(actual: u32, expected: u32) -> Result<(), ClientError> {
    if actual != expected {
        Err(ClientError::WrongLocalUser)
    } else {
        Ok(())
    }
}
