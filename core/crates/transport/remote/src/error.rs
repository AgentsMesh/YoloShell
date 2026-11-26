use crate::protocol::{ErrorBody, ProtocolError};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteFilesystemOperation {
    InspectMetadata,
    InspectLink,
    CreateDirectory,
    SetPermissions,
    CheckExistence,
    CreateFile,
    OpenFile,
    ReadFile,
    WriteFile,
    FlushFile,
    SyncFile,
    CloseFile,
    Rename,
    Remove,
    ReadLink,
    CreateLink,
    CloseSession,
    AtomicRename,
}

impl std::fmt::Display for RemoteFilesystemOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::InspectMetadata => "inspect metadata",
            Self::InspectLink => "inspect symbolic link",
            Self::CreateDirectory => "create directory",
            Self::SetPermissions => "set permissions",
            Self::CheckExistence => "check existence",
            Self::CreateFile => "create file",
            Self::OpenFile => "open file",
            Self::ReadFile => "read file",
            Self::WriteFile => "write file",
            Self::FlushFile => "flush file",
            Self::SyncFile => "sync file",
            Self::CloseFile => "close file",
            Self::Rename => "rename path",
            Self::Remove => "remove path",
            Self::ReadLink => "read symbolic link",
            Self::CreateLink => "create symbolic link",
            Self::CloseSession => "close SFTP session",
            Self::AtomicRename => "atomically rename path",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("bundle stage {stage} retained for native inspection: {cause}")]
    BundleStageRetained {
        stage: super::RetainedBundleStage,
        #[source]
        cause: Box<RemoteError>,
    },
    #[error("invalid remote endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("invalid SSH host key: {0}")]
    InvalidHostKey(String),
    #[error("malformed known_hosts entry at line {line}")]
    MalformedKnownHosts { line: usize },
    #[error("SSH host key is not trusted for {host}")]
    UnknownHostKey { host: String },
    #[error("SSH host key changed for {host}")]
    ChangedHostKey { host: String },
    #[error("SSH host key is revoked for {host}")]
    RevokedHostKey { host: String },
    #[error("invalid remote runtime configuration: {0}")]
    InvalidRuntime(String),
    #[error("remote runtime is incompatible: {0}")]
    RuntimeIncompatible(String),
    #[error("invalid remote runtime artifact: {0}")]
    InvalidArtifact(String),
    #[error("remote runtime integrity check failed: {0}")]
    RuntimeIntegrity(String),
    #[error("remote runtime deployment failed: {0}")]
    RuntimeDeployment(String),
    #[error("remote filesystem {operation} failed for {path}: {detail}")]
    RemoteFilesystem {
        operation: RemoteFilesystemOperation,
        path: PathBuf,
        detail: String,
    },
    #[error("{primary}; SFTP lifecycle settlement also failed: {settlement}")]
    LifecycleSettlement {
        primary: Box<RemoteError>,
        settlement: Box<RemoteError>,
    },
    #[error("remote transport is unavailable: {0}")]
    Transport(String),
    #[error("SSH authentication failed for {0}")]
    AuthenticationFailed(String),
    #[error("remote channel does not belong to the requested connection or purpose")]
    ChannelMismatch,
    #[error("remote rejected the SSH channel request")]
    ChannelRejected,
    #[error("remote command is invalid: {0}")]
    InvalidCommand(String),
    #[error("remote command is not allowed: {0}")]
    CommandDenied(String),
    #[error("remote command or file result exceeded its configured limit")]
    LimitExceeded,
    #[error("remote path is outside the configured read roots: {0}")]
    PathDenied(PathBuf),
    #[error("remote host is disconnected")]
    Disconnected,
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("remote host error: {0:?}")]
    Host(ErrorBody),
    #[error("known_hosts I/O error: {0}")]
    KnownHostsIo(#[from] std::io::Error),
}

impl RemoteError {
    pub fn transport(message: impl Into<String>) -> Self {
        Self::Transport(message.into())
    }

    pub fn is_connection_loss(&self) -> bool {
        match self {
            Self::BundleStageRetained { cause, .. } => cause.is_connection_loss(),
            Self::Transport(_)
            | Self::Protocol(
                ProtocolError::Io(_)
                | ProtocolError::Decode(_)
                | ProtocolError::FrameTooLarge { .. }
                | ProtocolError::EndOfStream
                | ProtocolError::ResponseMismatch
                | ProtocolError::EventMismatch
                | ProtocolError::ConnectionPoisoned,
            ) => true,
            Self::LifecycleSettlement {
                primary,
                settlement: _,
            } => primary.is_connection_loss(),
            _ => false,
        }
    }

    pub fn requires_connection_recovery(&self) -> bool {
        match self {
            Self::BundleStageRetained { cause, .. } => cause.requires_connection_recovery(),
            Self::LifecycleSettlement {
                primary,
                settlement,
            } => {
                primary.requires_connection_recovery() || settlement.requires_connection_recovery()
            }
            _ => self.is_connection_loss(),
        }
    }
}
