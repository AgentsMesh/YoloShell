use super::*;
use std::path::{Path, PathBuf};
use yolo_distribution_bundle::ArtifactRole;
impl MultiplexedSshConnection for Fake {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(self.command(command))
    }
    fn verify_bundle_executable<'a>(
        &'a self,
        request: &'a BundleStageRequest,
        location: BundleExecutableLocation,
        role: ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move {
            self.bundle
                .verify_bundle_executable(request, location, role)
                .await
                .map(|value| self.rebind(value))
        })
    }
    fn stage_bundle<'a>(
        &'a self,
        request: &'a BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<BundleStageResult>> {
        Box::pin(async move {
            self.bundle
                .stage_bundle(request)
                .await
                .map(|value| self.rebind(value))
        })
    }
    fn read_bundle_selection<'a>(
        &'a self,
        path: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>> {
        Box::pin(async move {
            self.bundle
                .read_bundle_selection(path)
                .await
                .map(|value| self.rebind(value))
        })
    }
    fn open_runtime<'a>(
        &'a self,
        _: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async { panic!() })
    }
    fn open_hub_peer<'a>(
        &'a self,
        _: &'a HubPeerRequest,
    ) -> RemoteFuture<'a, OpenedHubPeerChannel> {
        Box::pin(async { panic!() })
    }
    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async { panic!() })
    }
    fn read_file<'a>(
        &'a self,
        _: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async { panic!() })
    }
    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async { panic!() })
    }
    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async { panic!() })
    }
}
