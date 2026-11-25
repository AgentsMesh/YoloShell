#[path = "russh_backend/agent_identities.rs"]
mod agent_identities;
#[path = "russh_backend/approved_channel.rs"]
mod approved_channel;
#[path = "russh_backend/authentication.rs"]
mod authentication;
#[path = "russh_backend/bundle/bundle_cancel.rs"]
mod bundle_cancel;
#[path = "russh_backend/bundle/bundle_directory.rs"]
mod bundle_directory;
#[path = "russh_backend/bundle/bundle_executable.rs"]
mod bundle_executable;
#[path = "russh_backend/bundle/bundle_file.rs"]
mod bundle_file;
#[path = "russh_backend/bundle/bundle_identity.rs"]
mod bundle_identity;
#[path = "russh_backend/bundle/bundle_raw.rs"]
mod bundle_raw;
#[path = "russh_backend/bundle/bundle_selection.rs"]
mod bundle_selection;
#[path = "russh_backend/bundle/bundle_stage.rs"]
mod bundle_stage;
#[path = "russh_backend/bundle/bundle_task.rs"]
mod bundle_task;
#[path = "russh_backend/channel/channel_open_cleanup.rs"]
mod channel_open_cleanup;
#[path = "russh_backend/channel/channel_open_custody.rs"]
mod channel_open_custody;
#[path = "russh_backend/channel/channel_open_disconnect.rs"]
mod channel_open_disconnect;
#[path = "russh_backend/channel/channel_request.rs"]
mod channel_request;
#[path = "russh_backend/channel/channel_transfer_custody.rs"]
mod channel_transfer_custody;
#[path = "russh_backend/command_custody.rs"]
mod command_custody;
#[path = "russh_backend/command_line.rs"]
mod command_line;
#[path = "russh_backend/command_output.rs"]
mod command_output;
#[path = "russh_backend/config.rs"]
mod config;
#[path = "russh_backend/connect.rs"]
mod connect;
#[path = "russh_backend/deadline.rs"]
mod deadline;
#[path = "russh_backend/deployment.rs"]
mod deployment;
#[path = "russh_backend/deployment/deployment_activation.rs"]
mod deployment_activation;
#[path = "russh_backend/deployment/deployment_custody.rs"]
mod deployment_custody;
#[path = "russh_backend/deployment/deployment_fs.rs"]
mod deployment_fs;
#[path = "russh_backend/deployment/deployment_operation.rs"]
mod deployment_operation;
#[path = "russh_backend/deployment/deployment_verify.rs"]
mod deployment_verify;
#[path = "russh_backend/hub_peer.rs"]
mod hub_peer;
#[path = "russh_backend/hub_peer_driver.rs"]
mod hub_peer_driver;
#[path = "russh_backend/hub_peer_stream.rs"]
mod hub_peer_stream;
#[path = "russh_backend/identity_file.rs"]
mod identity_file;
pub use hub_peer_stream::{HubPeerClose, HubPeerStream};
#[path = "russh_backend/managed_raw_sftp_session.rs"]
mod managed_raw_sftp_session;
#[path = "russh_backend/managed_sftp_session.rs"]
mod managed_sftp_session;
#[path = "russh_backend/operations.rs"]
mod operations;
#[path = "russh_backend/posix_rename.rs"]
mod posix_rename;
#[path = "russh_backend/sftp.rs"]
mod sftp;
#[path = "russh_backend/sftp/sftp_capability.rs"]
mod sftp_capability;
#[path = "russh_backend/sftp/sftp_channel_driver.rs"]
mod sftp_channel_driver;
#[path = "russh_backend/sftp/sftp_file_identity.rs"]
mod sftp_file_identity;
#[path = "russh_backend/sftp/sftp_read.rs"]
mod sftp_read;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/russh_backend/connection_diagnostics.rs"]
mod connection_diagnostics;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/russh_backend/backend_tests.rs"]
#[cfg(test)]
mod backend_tests;
#[cfg(any(
    all(test, feature = "yolo-unit-tests"),
    feature = "broker-test-support"
))]
#[path = "../test_support/broker_command.rs"]
mod broker_command_support;
#[cfg(any(
    all(test, feature = "yolo-unit-tests"),
    feature = "broker-test-support"
))]
pub use broker_command_support::{
    BROKER_COMMAND_CANCELLATION_PROGRAM, BrokerCommandCleanupFixture, HubPeerTarget,
};

use super::constants::{
    FIRST_CHANNEL_ID, SSH_CONNECT_TIMEOUT, SSH_INACTIVITY_TIMEOUT, SSH_KEEPALIVE_INTERVAL,
    SSH_KEEPALIVE_MAX_FAILURES, SSH_OPERATION_TIMEOUT,
};
use super::*;
use config::AgentSource;
pub use config::SshConnectConfig;
pub use identity_file::IdentityFile;
use russh::client;
use russh::keys::PublicKey;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct RusshConnection {
    info: ConnectionInfo,
    session: Arc<client::Handle<HostKeyHandler>>,
    next_channel_id: AtomicU64,
    operation_timeout: Duration,
    cleanup: RemoteCleanupRegistry,
    authenticated_uid: tokio::sync::OnceCell<u32>,
}

impl RusshConnection {
    fn identity(&self, purpose: ChannelPurpose) -> ChannelIdentity {
        ChannelIdentity {
            connection_id: self.info.connection_id,
            channel_id: self.next_channel_id.fetch_add(1, Ordering::Relaxed),
            purpose,
        }
    }
}

include!("russh_backend/connection_cleanup.rs");

struct HostKeyHandler {
    endpoint: RemoteEndpoint,
    known_hosts: KnownHosts,
    presented: Arc<Mutex<Option<HostKey>>>,
}

impl client::Handler for HostKeyHandler {
    type Error = HandlerError;

    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        let openssh = key
            .to_openssh()
            .map_err(|error| HandlerError::Key(error.to_string()))?;
        let presented = HostKey::from_openssh(&openssh)?;
        self.known_hosts.verify(&self.endpoint, &presented)?;
        *self.presented.lock().unwrap() = Some(presented);
        Ok(true)
    }

    #[cfg(all(test, feature = "yolo-integration-tests"))]
    connection_diagnostics::disconnected_handler!();
}

#[derive(Debug, thiserror::Error)]
pub(super) enum HandlerError {
    #[error(transparent)]
    Ssh(#[from] russh::Error),
    #[error(transparent)]
    Remote(#[from] RemoteError),
    #[error("SSH key encoding failed: {0}")]
    Key(String),
}

fn ssh_error(error: impl std::fmt::Display) -> RemoteError {
    RemoteError::transport(error.to_string())
}
