use crate::ProvisionGuard;
use std::path::{Path, PathBuf};
use yolo_distribution_bundle::ArtifactRole;
use yolo_remote::*;

pub(crate) struct FencedConnection<'a> {
    pub inner: &'a dyn MultiplexedSshConnection,
    pub guard: &'a dyn ProvisionGuard,
}
macro_rules! guarded {
    ($this:expr, $operation:expr) => {
        Box::pin(async move {
            $this
                .guard
                .verify()
                .map_err(|_| RemoteError::ChannelMismatch)?;
            let result = $operation.await;
            $this
                .guard
                .verify()
                .map_err(|_| RemoteError::ChannelMismatch)?;
            result
        })
    };
}
impl MultiplexedSshConnection for FencedConnection<'_> {
    fn info(&self) -> &ConnectionInfo {
        self.inner.info()
    }
    fn verify_bundle_executable<'a>(
        &'a self,
        request: &'a BundleStageRequest,
        location: BundleExecutableLocation,
        role: ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        guarded!(
            self,
            self.inner.verify_bundle_executable(request, location, role)
        )
    }
    fn stage_bundle<'a>(
        &'a self,
        request: &'a BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<BundleStageResult>> {
        guarded!(self, self.inner.stage_bundle(request))
    }
    fn read_bundle_selection<'a>(
        &'a self,
        root: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>> {
        guarded!(self, self.inner.read_bundle_selection(root))
    }
    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        guarded!(self, self.inner.execute(command))
    }
    fn open_runtime<'a>(
        &'a self,
        _: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
    fn open_hub_peer<'a>(
        &'a self,
        _: &'a HubPeerRequest,
    ) -> RemoteFuture<'a, OpenedHubPeerChannel> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
    fn read_file<'a>(
        &'a self,
        _: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async { Err(RemoteError::ChannelMismatch) })
    }
}
