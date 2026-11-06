use super::*;
use std::path::Path;

impl MultiplexedSshConnection for Fake {
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }
    fn verify_bundle_executable<'a>(
        &'a self,
        request: &'a BundleStageRequest,
        location: BundleExecutableLocation,
        role: ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move {
            let mut identity = self.identity(ChannelPurpose::FileRead);
            let mut state = self.state.lock().unwrap();
            state.verifications += 1;
            let path = match location {
                BundleExecutableLocation::Staged => request.executable(role),
                BundleExecutableLocation::Published => request
                    .descriptor()
                    .executable_path(request.install_root(), role),
            };
            if matches!(state.fault, Fault::UnverifiedStaged(bad) if bad == role)
                || (matches!(state.fault, Fault::CorruptRecovery) && state.committed.is_some())
                || state
                    .files
                    .get(&path)
                    .is_none_or(|bytes| bytes != request.payload(role))
            {
                return Err(RemoteError::PathDenied(path));
            }
            if matches!(state.fault, Fault::WrongVerificationChannel) {
                identity.connection_id = Uuid::new_v4();
            }
            Ok(ChannelOutcome {
                identity,
                value: (),
            })
        })
    }
    fn stage_bundle<'a>(
        &'a self,
        request: &'a BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<BundleStageResult>> {
        Box::pin(async move {
            let identity = self.identity(ChannelPurpose::RuntimeDeployment);
            let mut state = self.state.lock().unwrap();
            state.stages += 1;
            if matches!(state.fault, Fault::StageFailure) {
                return Err(RemoteError::Transport(LOST_ACK.into()));
            }
            for role in ArtifactRole::ALL {
                state
                    .files
                    .insert(request.executable(role), request.payload(role).to_vec());
            }
            state.staged = Some(request.clone());
            let value = if matches!(state.fault, Fault::WrongStage) {
                let wrong = BundleStageRequest::new(
                    request.install_root().into(),
                    Uuid::new_v4(),
                    request.descriptor().clone(),
                    BundlePayloads {
                        runtime: request.payload(ArtifactRole::Runtime).into(),
                    },
                )
                .unwrap();
                BundleStageResult::from_request(&wrong)
            } else {
                BundleStageResult::from_request(request)
            };
            Ok(ChannelOutcome { identity, value })
        })
    }
    fn read_bundle_selection<'a>(
        &'a self,
        _: &'a Path,
    ) -> RemoteFuture<'a, ChannelOutcome<Option<Vec<u8>>>> {
        Box::pin(async move {
            let identity = self.identity(ChannelPurpose::FileRead);
            let state = self.state.lock().unwrap();
            let value = if matches!(state.fault, Fault::MalformedSelection) {
                Some(MALFORMED_JSON.into())
            } else {
                state
                    .selected
                    .as_ref()
                    .map(|selected| serde_json::to_vec(selected).unwrap())
            };
            Ok(ChannelOutcome { identity, value })
        })
    }
    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move {
            let identity = self.identity(ChannelPurpose::Command);
            let value = self.execute_command(command.request())?;
            Ok(ChannelOutcome { identity, value })
        })
    }
    fn read_file<'a>(
        &'a self,
        read: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async move {
            let identity = self.identity(ChannelPurpose::FileRead);
            let state = self.state.lock().unwrap();
            let bytes = state
                .files
                .get(read.resolved_path())
                .ok_or_else(|| RemoteError::PathDenied(read.resolved_path().into()))?;
            let start = (read.request().offset() as usize).min(bytes.len());
            let end = start
                .saturating_add(read.request().length() as usize)
                .min(bytes.len());
            let mut data = bytes[start..end].to_vec();
            if matches!(state.fault, Fault::CorruptRecovery) {
                if let Some(first) = data.first_mut() {
                    *first ^= MUTATION;
                }
            }
            let eof = data.len() as u64 != read.request().length();
            Ok(ChannelOutcome {
                identity,
                value: FileReadResult {
                    resolved_path: read.resolved_path().into(),
                    data,
                    eof,
                },
            })
        })
    }
    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn open_runtime<'a>(
        &'a self,
        _: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn open_hub_peer<'a>(
        &'a self,
        _: &'a HubPeerRequest,
    ) -> RemoteFuture<'a, OpenedHubPeerChannel> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
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
