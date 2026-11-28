use super::fixture;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use yolo_remote::{
    AuthorizedCommand, AuthorizedFileRead, ChannelIdentity, ChannelOutcome, ChannelPurpose,
    CommandOutput, ConnectionInfo, FileReadResult, HostKey, MultiplexedSshConnection,
    OpenedRemoteChannel, RemoteCleanupRegistry, RemoteEndpoint, RemoteError, RemoteFuture,
    RuntimeActivationRequest, RuntimeRole, RuntimeStageRequest, RuntimeStageResult,
};
use yolo_remote_broker::{BrokerConnectFuture, BrokerConnectionFactory};
#[path = "ready_remote_counts.rs"]
mod counts;
pub use counts::{Calls, counted_factory};

#[derive(Clone, Copy)]
enum FactoryBehavior {
    Ready,
    AuthenticationFailure,
}

struct TestFactory(FactoryBehavior);

pub fn factory() -> Arc<dyn BrokerConnectionFactory> {
    Arc::new(TestFactory(FactoryBehavior::Ready))
}

pub fn authentication_failure_factory() -> Arc<dyn BrokerConnectionFactory> {
    Arc::new(TestFactory(FactoryBehavior::AuthenticationFailure))
}

impl BrokerConnectionFactory for TestFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            match self.0 {
                FactoryBehavior::Ready => {
                    Ok(Arc::new(ReadyConnection::new()) as Arc<dyn MultiplexedSshConnection>)
                }
                FactoryBehavior::AuthenticationFailure => Err(RemoteError::AuthenticationFailed(
                    fixture::TEST_AUTHENTICATION_FAILURE.into(),
                )),
            }
        })
    }
}

struct ReadyConnection {
    calls: Arc<Calls>,
    info: ConnectionInfo,
    next_channel: AtomicU64,
}

const INITIAL_CHANNEL_ID: u64 = 1;
const CHANNEL_ID_INCREMENT: u64 = 1;

impl ReadyConnection {
    fn new() -> Self {
        Self {
            calls: Arc::new(Calls::default()),
            info: ConnectionInfo {
                connection_id: uuid::Uuid::new_v4(),
                endpoint: RemoteEndpoint::standard(fixture::TEST_USER, fixture::TEST_HOST).unwrap(),
                host_key: HostKey::new(
                    fixture::TEST_HOST_KEY_ALGORITHM,
                    fixture::TEST_HOST_KEY_ENCODED,
                )
                .unwrap(),
            },
            next_channel: AtomicU64::new(INITIAL_CHANNEL_ID),
        }
    }
}

impl MultiplexedSshConnection for ReadyConnection {
    fn verify_bundle_executable<'a>(
        &'a self,
        _: &'a yolo_remote::BundleStageRequest,
        _: yolo_remote::BundleExecutableLocation,
        _: yolo_remote::ArtifactRole,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn stage_bundle<'a>(
        &'a self,
        _: &'a yolo_remote::BundleStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<yolo_remote::BundleStageResult>> {
        self.calls.deployment();
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
        _: &'a yolo_remote::HubPeerRequest,
    ) -> RemoteFuture<'a, yolo_remote::OpenedHubPeerChannel> {
        Box::pin(async { Err(RemoteError::ChannelRejected) })
    }
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn open_runtime<'a>(
        &'a self,
        request: &'a yolo_remote::RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        self.calls.runtime();
        Box::pin(async move {
            let purpose = match request.role() {
                RuntimeRole::Control => ChannelPurpose::RuntimeControl,
                RuntimeRole::Attachment => ChannelPurpose::TerminalAttachment,
            };
            let identity = ChannelIdentity {
                connection_id: self.info.connection_id,
                channel_id: self
                    .next_channel
                    .fetch_add(CHANNEL_ID_INCREMENT, Ordering::Relaxed),
                purpose,
            };
            Ok(super::ready_runtime_channel::opened(identity))
        })
    }

    fn execute<'a>(
        &'a self,
        _: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        self.calls.command();
        rejected()
    }

    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        rejected()
    }

    fn read_file<'a>(
        &'a self,
        _: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        rejected()
    }

    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        self.calls.deployment();
        rejected()
    }

    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        self.calls.deployment();
        rejected()
    }
}

fn rejected<'a, T: Send + 'a>() -> RemoteFuture<'a, T> {
    Box::pin(async {
        Err(RemoteError::Transport(
            fixture::EXPECTED_CONNECTION_ERROR.into(),
        ))
    })
}
