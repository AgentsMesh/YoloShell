use super::support::{READ_PATH, connection_info};
use crate::remote::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
#[path = "../../test_support/unsupported_extensions.rs"]
mod unsupported_extensions;
use unsupported_extensions::unsupported_remote_extensions;

#[derive(Clone, Copy)]
pub enum OperationFault {
    None,
    CommandTransport,
    CommandRejected,
    CommandConnectionMismatch,
    CommandPurposeMismatch,
    CommandInvalidOutput,
    CanonicalTransport,
    CanonicalRejected,
    CanonicalConnectionMismatch,
    CanonicalPurposeMismatch,
    CanonicalOutside,
    CanonicalUnsafe,
    ReadTransport,
    ReadLifecycleSettlement,
    ReadRejected,
    ReadConnectionMismatch,
    ReadPurposeMismatch,
    ReadInvalidOutput,
}

pub struct OperationConnection {
    info: ConnectionInfo,
    fault: OperationFault,
}

impl OperationConnection {
    pub fn new(fault: OperationFault) -> Arc<Self> {
        Arc::new(Self {
            info: connection_info(),
            fault,
        })
    }

    pub fn id(&self) -> Uuid {
        self.info.connection_id
    }

    fn identity(&self, purpose: ChannelPurpose) -> ChannelIdentity {
        ChannelIdentity {
            connection_id: self.info.connection_id,
            channel_id: 1,
            purpose,
        }
    }

    fn operation_identity(&self, command: bool) -> ChannelIdentity {
        let purpose = if command {
            ChannelPurpose::Command
        } else {
            ChannelPurpose::FileRead
        };
        let mut identity = self.identity(purpose);
        if matches!(
            self.fault,
            OperationFault::CommandConnectionMismatch
                | OperationFault::CanonicalConnectionMismatch
                | OperationFault::ReadConnectionMismatch
        ) {
            identity.connection_id = Uuid::nil();
        }
        if matches!(
            self.fault,
            OperationFault::CommandPurposeMismatch
                | OperationFault::CanonicalPurposeMismatch
                | OperationFault::ReadPurposeMismatch
        ) {
            identity.purpose = ChannelPurpose::RuntimeControl;
        }
        identity
    }
}

impl MultiplexedSshConnection for OperationConnection {
    unsupported_remote_extensions!();
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn open_runtime<'a>(
        &'a self,
        _: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }

    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move {
            match self.fault {
                OperationFault::CommandTransport => return Err(RemoteError::transport("lost")),
                OperationFault::CommandRejected => return Err(RemoteError::ChannelRejected),
                _ => {}
            }
            let size = if matches!(self.fault, OperationFault::CommandInvalidOutput) {
                command.request().output_limit() + 1
            } else {
                1
            };
            Ok(ChannelOutcome {
                identity: self.operation_identity(true),
                value: CommandOutput {
                    stdout: vec![0; size],
                    stderr: Vec::new(),
                    exit_status: Some(0),
                    timed_out: false,
                    truncated: false,
                },
            })
        })
    }

    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async move {
            match self.fault {
                OperationFault::CanonicalTransport => return Err(RemoteError::transport("lost")),
                OperationFault::CanonicalRejected => return Err(RemoteError::ChannelRejected),
                _ => {}
            }
            let value = match self.fault {
                OperationFault::CanonicalOutside => PathBuf::from("/etc/passwd"),
                OperationFault::CanonicalUnsafe => PathBuf::from("relative/path"),
                _ => PathBuf::from(READ_PATH),
            };
            Ok(ChannelOutcome {
                identity: self.operation_identity(false),
                value,
            })
        })
    }

    fn read_file<'a>(
        &'a self,
        read: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async move {
            match self.fault {
                OperationFault::ReadTransport => return Err(RemoteError::transport("lost")),
                OperationFault::ReadLifecycleSettlement => {
                    return Err(RemoteError::LifecycleSettlement {
                        primary: Box::new(RemoteError::PathDenied(PathBuf::from(READ_PATH))),
                        settlement: Box::new(RemoteError::transport(OPERATION_TIMEOUT_MESSAGE)),
                    });
                }
                OperationFault::ReadRejected => return Err(RemoteError::ChannelRejected),
                _ => {}
            }
            let size = if matches!(self.fault, OperationFault::ReadInvalidOutput) {
                read.request().length() as usize + 1
            } else {
                1
            };
            Ok(ChannelOutcome {
                identity: self.operation_identity(false),
                value: FileReadResult {
                    resolved_path: PathBuf::from(READ_PATH),
                    data: vec![0; size],
                    eof: true,
                },
            })
        })
    }

    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }

    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
}
