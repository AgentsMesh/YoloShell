use super::command_custody::CommandChannelCustody;
use super::command_line::{command_line, runtime_command};
use super::deadline::remote_operation;
use super::*;
use tokio::time::{Instant, timeout_at};

impl MultiplexedSshConnection for RusshConnection {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn verify_bundle_executable<'a>(
        &'a self,
        request: &'a BundleStageRequest,
        location: BundleExecutableLocation,
        role: yolo_distribution_bundle::ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move {
            self.verify_bundle_executable_sftp(request, location, role)
                .await
        })
    }

    fn stage_bundle<'a>(
        &'a self,
        request: &'a BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<BundleStageResult>> {
        Box::pin(async move { self.stage_bundle_sftp(request).await })
    }

    fn read_bundle_selection<'a>(
        &'a self,
        root: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>> {
        Box::pin(async move { self.read_bundle_selection_sftp(root).await })
    }

    fn open_hub_peer<'a>(
        &'a self,
        request: &'a HubPeerRequest,
    ) -> RemoteFuture<'a, OpenedHubPeerChannel> {
        Box::pin(async move { self.open_hub_peer_inner(request).await })
    }

    fn open_runtime<'a>(
        &'a self,
        request: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async move { self.open_runtime_inner(request).await })
    }

    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move { self.execute_inner(command).await })
    }

    fn canonicalize<'a>(&'a self, path: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async move { self.canonicalize_sftp(path).await })
    }

    fn read_file<'a>(
        &'a self,
        read: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async move { self.read_sftp(read).await })
    }

    fn stage_runtime<'a>(
        &'a self,
        request: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async move { self.stage_runtime_sftp(request).await })
    }

    fn activate_runtime<'a>(
        &'a self,
        request: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move { self.activate_runtime_sftp(request).await })
    }
}

impl RusshConnection {
    async fn open_runtime_inner(
        &self,
        request: &RuntimeChannelRequest,
    ) -> Result<OpenedRemoteChannel, RemoteError> {
        let command = runtime_command(&request.runtime)?;
        let mut channel = self.open_session_channel().await?;
        let approved = remote_operation(
            self.operation_timeout,
            super::channel_request::exec(channel.channel(), command),
        )
        .await;
        let approved = approved?;
        let purpose = match request.role() {
            RuntimeRole::Control => ChannelPurpose::RuntimeControl,
            RuntimeRole::Attachment => ChannelPurpose::TerminalAttachment,
        };
        Ok(OpenedRemoteChannel {
            identity: self.identity(purpose),
            stream: Box::new(approved.into_stream(channel.release())),
        })
    }

    async fn execute_inner(
        &self,
        command: &AuthorizedCommand,
    ) -> Result<ChannelOutcome<CommandOutput>, RemoteError> {
        let deadline = Instant::now() + command.request().timeout();
        let identity = self.identity(ChannelPurpose::Command);
        let opened = self.open_session_channel_until(deadline).await?;
        let (session, channel, reservation) = opened.into_parts();
        let mut custody =
            CommandChannelCustody::new(session, channel, self.operation_timeout, reservation);
        let command_text = command_line(command)?;
        let approved = match timeout_at(
            deadline,
            super::channel_request::exec(custody.channel(), command_text),
        )
        .await
        {
            Ok(Ok(approved)) => approved,
            Ok(Err(error)) => return Err(error),
            Err(_) => {
                return Ok(ChannelOutcome {
                    identity,
                    value: super::command_output::timed_out(),
                });
            }
        };
        let value =
            super::command_output::collect(custody.channel(), approved, command, deadline).await?;
        if !value.timed_out {
            custody.release();
        }
        Ok(ChannelOutcome { identity, value })
    }
}
