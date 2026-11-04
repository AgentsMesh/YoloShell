use super::deployment::{RuntimeActivationRequest, RuntimeStageRequest, RuntimeStageResult};
use super::{
    AuthorizedCommand, AuthorizedFileRead, CommandOutput, ConnectionInfo, FileReadResult,
    RemoteError, RuntimeChannelRequest,
};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelPurpose {
    RuntimeControl,
    TerminalAttachment,
    Command,
    FileRead,
    RuntimeDeployment,
    HubPeer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelIdentity {
    pub connection_id: Uuid,
    pub channel_id: u64,
    pub purpose: ChannelPurpose,
}

#[derive(Debug)]
pub struct ChannelOutcome<T> {
    pub identity: ChannelIdentity,
    pub value: T,
}

pub trait RemoteChannelIo: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> RemoteChannelIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}
pub type BoxedRemoteChannel = Box<dyn RemoteChannelIo>;

pub struct OpenedRemoteChannel {
    pub identity: ChannelIdentity,
    pub stream: BoxedRemoteChannel,
}

pub type RemoteFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, RemoteError>> + Send + 'a>>;

/// Every operation opens a distinct SSH channel on this one authenticated transport.
/// Dropping any channel must not disconnect the transport or terminate a remote worker.
pub trait MultiplexedSshConnection: Send + Sync {
    fn info(&self) -> &ConnectionInfo;

    fn verify_bundle_executable<'a>(
        &'a self,
        request: &'a super::BundleStageRequest,
        location: super::BundleExecutableLocation,
        role: yolo_distribution_bundle::ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>>;

    fn stage_bundle<'a>(
        &'a self,
        request: &'a super::BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<super::BundleStageResult>>;

    fn read_bundle_selection<'a>(
        &'a self,
        root: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>>;

    fn open_runtime<'a>(
        &'a self,
        request: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel>;

    fn open_hub_peer<'a>(
        &'a self,
        request: &'a super::HubPeerRequest,
    ) -> RemoteFuture<'a, super::OpenedHubPeerChannel>;

    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>>;

    fn canonicalize<'a>(&'a self, path: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>>;

    fn read_file<'a>(
        &'a self,
        read: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>>;

    fn stage_runtime<'a>(
        &'a self,
        request: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>>;

    fn activate_runtime<'a>(
        &'a self,
        request: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>>;
}

pub(crate) fn validate_channel(
    actual: ChannelIdentity,
    connection_id: Uuid,
    purpose: ChannelPurpose,
) -> Result<(), RemoteError> {
    if actual.connection_id != connection_id || actual.purpose != purpose {
        return Err(RemoteError::ChannelMismatch);
    }
    Ok(())
}
