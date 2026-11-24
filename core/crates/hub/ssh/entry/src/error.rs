use std::io;
use yolo_hub_client::ClientError;
use yolo_hub_jsonrpc_transport::TransportError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("unsupported SSH entry profile schema")]
    Schema,
    #[error("invalid SSH entry host")]
    Host,
    #[error("invalid SSH entry port")]
    Port,
    #[error("invalid SSH entry account name")]
    Username,
    #[error("invalid SSH entry host key pin")]
    HostKey,
    #[error("invalid SSH public entry socket")]
    Socket,
    #[error("invalid SSH public entry Hub key pin")]
    HubPin,
    #[error("invalid SSH public client identity")]
    ClientId,
    #[error("invalid SSH entry profile encoding")]
    Encoding,
    #[error("SSH entry profile exceeds its size limit")]
    Size,
    #[error("invalid SSH entry credential key type")]
    Credential,
}

pub(crate) fn transport(kind: io::ErrorKind, message: &'static str) -> ClientError {
    ClientError::Transport(TransportError::Io(io::Error::new(kind, message)))
}
pub(crate) fn timeout(_: tokio::time::error::Elapsed) -> ClientError {
    ClientError::Transport(TransportError::Deadline)
}
pub(crate) fn stage(
    deadline: tokio::time::Instant,
    kind: io::ErrorKind,
    message: &'static str,
) -> ClientError {
    if tokio::time::Instant::now() >= deadline {
        ClientError::Transport(TransportError::Deadline)
    } else {
        transport(kind, message)
    }
}
