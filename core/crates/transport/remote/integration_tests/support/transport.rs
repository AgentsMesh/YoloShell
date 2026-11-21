use super::server::serve_runtime;
use super::*;
use std::path::Path;

impl MultiplexedSshConnection for FakeConnection {
    fn verify_bundle_executable<'a>(
        &'a self,
        _: &'a crate::remote::BundleStageRequest,
        _: crate::remote::BundleExecutableLocation,
        _: crate::remote::ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn stage_bundle<'a>(
        &'a self,
        _: &'a crate::remote::BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<crate::remote::BundleStageResult>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn read_bundle_selection<'a>(
        &'a self,
        _: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn open_hub_peer<'a>(
        &'a self,
        _: &'a crate::remote::HubPeerRequest,
    ) -> RemoteFuture<'a, crate::remote::OpenedHubPeerChannel> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn open_runtime<'a>(
        &'a self,
        request: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async move {
            let purpose = match request.role() {
                RuntimeRole::Control => ChannelPurpose::RuntimeControl,
                RuntimeRole::Attachment => ChannelPurpose::TerminalAttachment,
            };
            let identity = self.identity(purpose);
            let behavior = self.next_runtime_behavior();
            if let RuntimeBehavior::DelayOpenThenStallAttachment { delay, .. } = &behavior {
                tokio::time::sleep(*delay).await;
            }
            let (client, server) = tokio::io::duplex(TEST_CHANNEL_CAPACITY);
            let break_control = self.break_control_after_handshake.load(Ordering::Relaxed);
            tokio::spawn(serve_runtime(
                server,
                request.role,
                self.status(),
                break_control,
                behavior,
                self.requests.clone(),
            ));
            Ok(OpenedRemoteChannel {
                identity,
                stream: Box::new(client),
            })
        })
    }

    fn execute<'a>(
        &'a self,
        _: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move {
            let value = self
                .command_outputs
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| self.command_output.lock().unwrap().clone());
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::Command),
                value,
            })
        })
    }

    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::FileRead),
                value: self.resolved_path.lock().unwrap().clone(),
            })
        })
    }

    fn read_file<'a>(
        &'a self,
        _: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::FileRead),
                value: self.file_result.lock().unwrap().clone(),
            })
        })
    }

    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::RuntimeDeployment),
                value: RuntimeStageResult {
                    already_active: false,
                    had_previous_runtime: false,
                },
            })
        })
    }

    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::RuntimeDeployment),
                value: (),
            })
        })
    }
}
