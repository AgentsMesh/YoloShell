#[path = "bundle.rs"]
mod bundle;
#[path = "bundle_retention.rs"]
mod bundle_retention;
#[path = "cleanup_registry.rs"]
mod cleanup_registry;
#[path = "client.rs"]
mod client;
#[path = "command.rs"]
mod command;
#[path = "constants.rs"]
pub(crate) mod constants;
#[path = "deadline.rs"]
mod deadline;
#[path = "deployment.rs"]
mod deployment;
#[path = "error.rs"]
mod error;
#[path = "file.rs"]
mod file;
#[path = "host_key.rs"]
mod host_key;
#[path = "host_key_match.rs"]
mod host_key_match;
#[path = "hub_peer.rs"]
mod hub_peer;
#[path = "local_file.rs"]
mod local_file;
#[path = "protocol.rs"]
mod protocol;
#[path = "rpc.rs"]
mod rpc;
#[path = "russh_backend.rs"]
mod russh_backend;
#[path = "transport.rs"]
mod transport;
#[path = "types.rs"]
mod types;

pub use bundle::{BundleExecutableLocation, BundlePayloads, BundleStageRequest, BundleStageResult};
pub use bundle_retention::{
    BundleRetentionReason, MAX_RETAINED_BUNDLE_STAGES, RetainedBundleStage,
};
#[doc(hidden)]
pub use cleanup_registry::RemoteCleanupRegistry;
#[cfg(not(feature = "broker-test-support"))]
pub(crate) use cleanup_registry::RemoteCleanupReservation;
#[cfg(feature = "broker-test-support")]
#[doc(hidden)]
pub use cleanup_registry::RemoteCleanupReservation;
#[doc(hidden)]
pub use cleanup_registry::cleanup_settlement_timeout;
pub use client::{RemoteAttachment, RemoteHost};
pub use command::{AuthorizedCommand, CommandOutput, CommandPolicy, CommandRequest};
#[cfg(any(test, feature = "broker-test-support"))]
#[doc(hidden)]
pub use constants::{
    ARCHITECTURE_ARGUMENT, PLATFORM_ARGUMENT, REQUIRED_RUNTIME_CAPABILITIES,
    RUNTIME_VERSION_ARGUMENT,
};
pub use constants::{OPERATION_TIMEOUT_MESSAGE, SSH_OPERATION_TIMEOUT};
#[doc(hidden)]
pub use deadline::RemoteDeadline;
pub use deployment::{
    RemoteArchitecture, RemotePlatform, RuntimeActivationRequest, RuntimeArtifact,
    RuntimeDeploymentResult, RuntimeInstaller, RuntimeStageRequest, RuntimeStageResult,
};
pub use error::{RemoteError, RemoteFilesystemOperation};
#[doc(hidden)]
pub use file::strict_absolute;
pub use file::{AuthorizedFileRead, FileReadPolicy, FileReadRequest, FileReadResult};
pub use host_key::KnownHosts;
pub use hub_peer::{HubPeerEndpoint, HubPeerRequest, OpenedHubPeerChannel};
pub use protocol::{
    ReconnectReport, RemoteRuntimeSpec, RemoteState, RuntimeChannelRequest, RuntimeIdentity,
    RuntimeRole,
};
#[doc(hidden)]
pub use rpc::probe_runtime_channel;
#[cfg(any(
    all(test, feature = "yolo-unit-tests"),
    feature = "broker-test-support"
))]
#[doc(hidden)]
pub use russh_backend::HubPeerTarget;
#[cfg(feature = "broker-test-support")]
#[doc(hidden)]
pub use russh_backend::{BROKER_COMMAND_CANCELLATION_PROGRAM, BrokerCommandCleanupFixture};
pub use russh_backend::{HubPeerClose, HubPeerStream};
pub use russh_backend::{IdentityFile, RusshConnection, SshConnectConfig};
pub use transport::{
    BoxedRemoteChannel, ChannelIdentity, ChannelOutcome, ChannelPurpose, MultiplexedSshConnection,
    OpenedRemoteChannel, RemoteChannelIo, RemoteFuture,
};
pub use types::{ConnectionInfo, HostKey, RemoteEndpoint};
pub use yolo_distribution_bundle::ArtifactRole;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/mod.rs"]
mod tests;
